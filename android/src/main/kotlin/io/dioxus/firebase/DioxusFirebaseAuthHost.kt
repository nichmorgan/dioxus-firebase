package io.dioxus.firebase

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.net.Uri
import android.os.CancellationSignal
import android.os.Handler
import android.os.Looper
import androidx.credentials.ClearCredentialStateRequest
import androidx.credentials.CredentialManager
import androidx.credentials.CredentialManagerCallback
import androidx.credentials.CustomCredential
import androidx.credentials.GetCredentialRequest
import androidx.credentials.GetCredentialResponse
import androidx.credentials.exceptions.ClearCredentialException
import androidx.credentials.exceptions.GetCredentialCancellationException
import androidx.credentials.exceptions.GetCredentialException
import androidx.credentials.exceptions.NoCredentialException
import com.google.android.gms.tasks.Task
import com.google.android.gms.tasks.Tasks
import com.google.android.libraries.identity.googleid.GetGoogleIdOption
import com.google.android.libraries.identity.googleid.GetSignInWithGoogleOption
import com.google.android.libraries.identity.googleid.GoogleIdTokenCredential
import com.google.firebase.FirebaseApp
import com.google.firebase.FirebaseOptions
import com.google.firebase.auth.AuthCredential
import com.google.firebase.auth.AuthResult
import com.google.firebase.auth.FirebaseAuth
import com.google.firebase.auth.FirebaseAuthException
import com.google.firebase.auth.FirebaseAuthRecentLoginRequiredException
import com.google.firebase.auth.FirebaseUser
import com.google.firebase.auth.GoogleAuthProvider
import com.google.firebase.auth.OAuthCredential
import com.google.firebase.auth.OAuthProvider
import com.google.firebase.auth.UserProfileChangeRequest
import java.lang.ref.WeakReference
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

/**
 * Thin Kotlin host so Rust can call Firebase Auth via JNI.
 *
 * Reply protocol (matches Rust `host_protocol`):
 * - success void: `OK`
 * - success user: `OK\u001fuid\u001femail\u001fdisplayName\u001fphotoURL\u001fN`
 *   then N times `providerId\u001fuid\u001femail\u001fdisplayName\u001fphotoURL`
 * - success optional string: `OK` or `OK\u001fvalue`
 * - error: `ERR\u001f<code>\u001f<message>`
 *
 * Top-level fields are Firebase user values as-is. Every `providerData` row is
 * serialized (including `firebase` when present). No Google/Apple merge.
 */
object DioxusFirebaseAuthHost {
    private const val UNIT = '\u001f'
    /** Custom Tabs / Apple sheets need longer than email password ops. */
    private const val TASK_TIMEOUT_SECONDS = 300L
    private val subscribed = AtomicBoolean(false)
    private val activityRef = AtomicReference<WeakReference<Activity>?>(null)
    @Volatile
    private var appContext: Context? = null
    @Volatile
    private var googleWebClientId: String = ""

    private val auth: FirebaseAuth
        get() = FirebaseAuth.getInstance()

    // --- Public JNI API ---

    @JvmStatic
    fun initializeApp(
        context: Context,
        apiKey: String,
        appId: String,
        projectId: String,
        messagingSenderId: String,
        storageBucket: String,
        databaseUrl: String,
        googleWebClientId: String,
    ): String = safeCall {
        this.googleWebClientId = googleWebClientId.trim()
        this.appContext = context.applicationContext
        if (context is Activity) {
            activityRef.set(WeakReference(context))
        } else if (context is ContextWrapper) {
            var walk: Context? = context
            while (walk is ContextWrapper) {
                if (walk is Activity) {
                    activityRef.set(WeakReference(walk))
                    break
                }
                walk = walk.baseContext
            }
        }
        runOnMainSync {
            if (FirebaseApp.getApps(context).isEmpty()) {
                val builder = FirebaseOptions.Builder()
                    .setApiKey(apiKey.trim())
                    .setApplicationId(appId.trim())
                    .setProjectId(projectId.trim())
                if (messagingSenderId.isNotBlank()) builder.setGcmSenderId(messagingSenderId.trim())
                if (storageBucket.isNotBlank()) builder.setStorageBucket(storageBucket.trim())
                if (databaseUrl.isNotBlank()) builder.setDatabaseUrl(databaseUrl.trim())

                FirebaseApp.initializeApp(context.applicationContext, builder.build())
            }
            auth // Ensure instance initialized
            ok()
        }
    }

    @JvmStatic
    fun signInWithEmailAndPassword(email: String, password: String): String =
        awaitTask({ auth.signInWithEmailAndPassword(email.trim(), password) }) { okUser(it.user) }

    @JvmStatic
    fun createUserWithEmailAndPassword(email: String, password: String): String =
        awaitTask({ auth.createUserWithEmailAndPassword(email.trim(), password) }) { okUser(it.user) }

    @JvmStatic
    fun updateDisplayName(displayName: String): String =
        updateProfile(displayName = displayName, photoUrl = null)

    @JvmStatic
    fun updatePhotoURL(photoURL: String): String =
        updateProfile(displayName = null, photoUrl = photoURL)

    @JvmStatic
    fun sendPasswordResetEmail(email: String): String =
        awaitTask({ auth.sendPasswordResetEmail(email.trim()) }) { ok() }

    @JvmStatic
    fun signOut(): String = safeCall {
        auth.signOut()
        clearCredentialStateBestEffort()
        ok()
    }

    @JvmStatic
    fun currentUser(): String = safeCall {
        okUser(auth.currentUser)
    }

    @JvmStatic
    fun getIdToken(forceRefresh: Boolean): String {
        val user = auth.currentUser ?: return ok()
        return awaitTask({ user.getIdToken(forceRefresh) }) { result ->
            val token = result?.token
            if (token.isNullOrEmpty()) ok() else "OK$UNIT$token"
        }
    }

    @JvmStatic
    fun useAuthEmulator(host: String, port: Int): String = safeCall {
        auth.useEmulator(host.trim(), port)
        ok()
    }

    @JvmStatic
    fun authStateChanges(): String {
        if (!subscribed.compareAndSet(false, true)) {
            onAuthStateChanged(auth.currentUser?.let(::userFields))
            return ok()
        }
        return safeCall {
            auth.addAuthStateListener { a ->
                onAuthStateChanged(a.currentUser?.let(::userFields))
            }
            ok()
        }
    }

    @JvmStatic
    fun signInWithGoogle(): String = signInOrLinkWithGoogle(link = false)

    @JvmStatic
    fun linkWithGoogle(): String = signInOrLinkWithGoogle(link = true)

    /**
     * Show the Google account sheet and reply `OK␟idToken␟` without touching
     * Firebase. Apps send the token to a backend that links it server-side.
     */
    @JvmStatic
    fun pickGoogleCredential(): String = when (val result = pickGoogleIdToken()) {
        is CredentialResult.Ok -> "OK$UNIT${result.idToken}$UNIT"
        is CredentialResult.Err -> result.reply
    }

    /** Reload the current user and force a fresh ID token (server-side provider changes). */
    @JvmStatic
    fun reloadCurrentUser(): String {
        val user = auth.currentUser ?: return ok()
        return awaitTask({
            Tasks.await(user.reload(), TASK_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            user.getIdToken(true)
        }) { okUser(auth.currentUser) }
    }

    @JvmStatic
    fun signInWithApple(): String = signInOrLinkWithApple(link = false)

    @JvmStatic
    fun linkWithApple(): String = signInOrLinkWithApple(link = true)

    /** Unlink a provider id (e.g. `google.com`). */
    @JvmStatic
    fun unlink(providerId: String): String {
        val user = auth.currentUser ?: return noCurrentUser()
        val id = providerId.trim()
        if (id.isEmpty()) {
            return err("invalid-argument", "provider_id must not be empty")
        }
        return awaitTask({ user.unlink(id) }) { okUser(it.user) }
    }

    /**
     * Reauthenticate with the `google.com` or `apple.com` sheet, then delete the
     * current user. A dismissed sheet replies before Firebase is touched.
     */
    @JvmStatic
    fun deleteCurrentUser(providerId: String): String {
        val user = auth.currentUser ?: return noCurrentUser()
        return when (providerId.trim()) {
            "google.com" -> reauthenticateWithGoogleAndDelete(user)
            "apple.com" -> reauthenticateWithAppleAndDelete(user)
            else -> err("invalid-argument", "provider_id must be google.com or apple.com")
        }
    }

    /** Implemented in Rust via [JNIEnv.RegisterNatives]. */
    @JvmStatic
    external fun onAuthStateChanged(userPayload: String?)

    // --- Social helpers ---

    private fun signInOrLinkWithGoogle(link: Boolean): String {
        val credential = when (val result = pickGoogleIdToken()) {
            is CredentialResult.Ok -> GoogleAuthProvider.getCredential(result.idToken, null)
            is CredentialResult.Err -> return result.reply
        }
        return applyCredential(credential, link)
    }

    /** `reauth` first offers only accounts already authorized for this app, auto-selected. */
    private fun pickGoogleIdToken(reauth: Boolean = false): CredentialResult {
        if (Looper.myLooper() == Looper.getMainLooper()) {
            return CredentialResult.Err(
                err("native", "Firebase Auth waits must not run on the main looper; use spawn_blocking"),
            )
        }
        val clientId = googleWebClientId
        if (clientId.isBlank()) {
            return CredentialResult.Err(err("unavailable", "google_web_client_id is required for Google sign-in"))
        }
        val activity = requireActivity()
            ?: return CredentialResult.Err(err("unavailable", "Activity required for Google sign-in"))
        if (reauth) {
            val authorized = GetGoogleIdOption.Builder()
                .setFilterByAuthorizedAccounts(true)
                .setServerClientId(clientId)
                .setAutoSelectEnabled(true)
                .build()
            val request = GetCredentialRequest.Builder()
                .addCredentialOption(authorized)
                .build()
            when (val first = getCredential(activity, request)) {
                is CredentialResult.Ok -> return first
                is CredentialResult.Err -> {
                    if (!shouldFallbackToSignInWithGoogle(first.reply)) {
                        return first
                    }
                }
            }
        }
        return awaitGoogleIdToken(activity, clientId)
    }

    private fun signInOrLinkWithApple(link: Boolean): String {
        if (Looper.myLooper() == Looper.getMainLooper()) {
            return err("native", "Firebase Auth waits must not run on the main looper; use spawn_blocking")
        }
        val activity = requireActivity() ?: return err("unavailable", "Activity required for Apple sign-in")
        val provider = appleProvider()
        return try {
            val task = if (link) {
                val user = auth.currentUser ?: return noCurrentUser()
                user.startActivityForLinkWithProvider(activity, provider)
            } else {
                auth.startActivityForSignInWithProvider(activity, provider)
            }
            // The Custom Tab link finishes in GenericIdpActivity and can leave
            // currentUser without apple.com in providerData until a reload.
            awaitAuthResult(task, reloadUser = link)
        } catch (t: Throwable) {
            mapThrowable(t)
        }
    }

    private fun appleProvider(): OAuthProvider =
        OAuthProvider.newBuilder("apple.com")
            .setScopes(listOf("email", "name"))
            .build()

    /** Delete right away; open a sheet only when Firebase wants a recent sign-in. */
    private fun reauthenticateWithGoogleAndDelete(user: FirebaseUser): String {
        if (Looper.myLooper() == Looper.getMainLooper()) {
            return err("native", "Firebase Auth waits must not run on the main looper; use spawn_blocking")
        }
        try {
            Tasks.await(user.delete(), TASK_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            return ok()
        } catch (t: Throwable) {
            if (!requiresRecentLogin(t)) {
                return mapThrowable(t)
            }
        }
        val credential = when (val result = pickGoogleIdToken(reauth = true)) {
            is CredentialResult.Ok -> GoogleAuthProvider.getCredential(result.idToken, null)
            is CredentialResult.Err -> return result.reply
        }
        return awaitTask({
            Tasks.await(user.reauthenticate(credential), TASK_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            user.delete()
        }) { ok() }
    }

    private fun requiresRecentLogin(t: Throwable): Boolean {
        var cur: Throwable? = t
        while (cur != null) {
            if (cur is FirebaseAuthRecentLoginRequiredException) return true
            cur = cur.cause
        }
        return false
    }

    private fun reauthenticateWithAppleAndDelete(user: FirebaseUser): String {
        if (Looper.myLooper() == Looper.getMainLooper()) {
            return err("native", "Firebase Auth waits must not run on the main looper; use spawn_blocking")
        }
        val activity = requireActivity() ?: return err("unavailable", "Activity required for Apple sign-in")
        return awaitTask({
            val result = Tasks.await(
                user.startActivityForReauthenticateWithProvider(activity, appleProvider()),
                TASK_TIMEOUT_SECONDS,
                TimeUnit.SECONDS,
            )
            val accessToken = (result.credential as? OAuthCredential)?.accessToken
            if (!accessToken.isNullOrBlank()) {
                try {
                    Tasks.await(auth.revokeAccessToken(accessToken), TASK_TIMEOUT_SECONDS, TimeUnit.SECONDS)
                } catch (_: Throwable) {
                    // A failed revoke must not keep an account the user asked to delete.
                }
            }
            user.delete()
        }) { ok() }
    }

    /** Best-effort Credential Manager clear; must not fail Firebase sign-out. */
    private fun clearCredentialStateBestEffort() {
        val context = requireActivity() ?: appContext ?: return
        val latch = CountDownLatch(1)
        val manager = CredentialManager.create(context)
        val executor = Executors.newSingleThreadExecutor()
        try {
            manager.clearCredentialStateAsync(
                ClearCredentialStateRequest(),
                CancellationSignal(),
                executor,
                object : CredentialManagerCallback<Void?, ClearCredentialException> {
                    override fun onResult(result: Void?) {
                        latch.countDown()
                    }

                    override fun onError(e: ClearCredentialException) {
                        latch.countDown()
                    }
                },
            )
            latch.await(TASK_TIMEOUT_SECONDS, TimeUnit.SECONDS)
        } catch (_: Throwable) {
            // Ignore — Firebase sign-out already succeeded.
        } finally {
            executor.shutdown()
        }
    }

    private fun applyCredential(credential: AuthCredential, link: Boolean): String {
        return if (link) {
            val user = auth.currentUser ?: return noCurrentUser()
            awaitAuthResult(user.linkWithCredential(credential))
        } else {
            awaitTask({ auth.signInWithCredential(credential) }) { okUser(it.user) }
        }
    }

    /** Await an AuthResult task. Link collisions surface as Firebase wire codes. */
    private fun awaitAuthResult(task: Task<AuthResult>, reloadUser: Boolean = false): String {
        if (Looper.myLooper() == Looper.getMainLooper()) {
            return err("native", "Firebase Auth waits must not run on the main looper; use spawn_blocking")
        }
        return try {
            finalizeAuthUser(
                Tasks.await(task, TASK_TIMEOUT_SECONDS, TimeUnit.SECONDS),
                reloadUser,
            )
        } catch (t: Throwable) {
            mapThrowable(t)
        }
    }

    /**
     * Prefer Firebase's stored display name. On first Apple auth, AdditionalUserInfo
     * may carry a name Firebase did not copy — apply it once via updateProfile.
     * Apple only shares the name on the first authorization for that Services ID.
     * `reloadUser` refreshes providerData from the server before the reply.
     */
    private fun finalizeAuthUser(result: AuthResult, reloadUser: Boolean): String {
        val user = result.user ?: return ok()
        var reloaded = false
        val profileName = AppleProfileName.from(result.additionalUserInfo?.profile)
        if (user.displayName.isNullOrBlank() && !profileName.isNullOrBlank()) {
            try {
                Tasks.await(
                    user.updateProfile(
                        UserProfileChangeRequest.Builder().setDisplayName(profileName).build(),
                    ),
                    TASK_TIMEOUT_SECONDS,
                    TimeUnit.SECONDS,
                )
                Tasks.await(user.reload(), TASK_TIMEOUT_SECONDS, TimeUnit.SECONDS)
                reloaded = true
            } catch (_: Throwable) {
                // Best-effort; still return the signed-in user.
            }
        }
        val current = auth.currentUser ?: user
        if (reloadUser && !reloaded) {
            try {
                Tasks.await(current.reload(), TASK_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            } catch (_: Throwable) {
                // Best-effort; the link already succeeded.
            }
        }
        return okUser(auth.currentUser ?: current)
    }

    private fun requireActivity(): Activity? = activityRef.get()?.get()

    private sealed class CredentialResult {
        data class Ok(val idToken: String) : CredentialResult()
        data class Err(val reply: String) : CredentialResult()
    }

    private fun awaitGoogleIdToken(activity: Activity, serverClientId: String): CredentialResult {
        val googleIdOption = GetGoogleIdOption.Builder()
            .setFilterByAuthorizedAccounts(false)
            .setServerClientId(serverClientId)
            .setAutoSelectEnabled(false)
            .build()
        val oneTap = GetCredentialRequest.Builder()
            .addCredentialOption(googleIdOption)
            .build()

        when (val first = getCredential(activity, oneTap)) {
            is CredentialResult.Ok -> return first
            is CredentialResult.Err -> {
                if (!shouldFallbackToSignInWithGoogle(first.reply)) {
                    return first
                }
            }
        }

        // One Tap found no matching credential — show the Sign in with Google button flow.
        val signInOption = GetSignInWithGoogleOption.Builder(serverClientId).build()
        val buttonFlow = GetCredentialRequest.Builder()
            .addCredentialOption(signInOption)
            .build()
        return getCredential(activity, buttonFlow)
    }

    /** True when One Tap failed for lack of credentials (not user cancel). */
    private fun shouldFallbackToSignInWithGoogle(errReply: String): Boolean {
        // Wire form: ERR\u001f<code>\u001f<message>
        val parts = errReply.split('\u001f')
        if (parts.size < 3 || parts[0] != "ERR") return false
        if (parts[1] == "canceled") return false
        val message = parts[2]
        return message.contains("Cannot find a matching credential", ignoreCase = true) ||
            message.contains("No credentials available", ignoreCase = true) ||
            message.contains("[28433]") ||
            Regex("""\b16\b""").containsMatchIn(message)
    }

    private fun getCredential(activity: Activity, request: GetCredentialRequest): CredentialResult {
        val latch = CountDownLatch(1)
        val result = AtomicReference<CredentialResult>()
        val manager = CredentialManager.create(activity)
        val executor = Executors.newSingleThreadExecutor()
        try {
            manager.getCredentialAsync(
                activity,
                request,
                CancellationSignal(),
                executor,
                object : CredentialManagerCallback<GetCredentialResponse, GetCredentialException> {
                    override fun onResult(response: GetCredentialResponse) {
                        result.set(parseGoogleCredentialResponse(response))
                        latch.countDown()
                    }

                    override fun onError(e: GetCredentialException) {
                        result.set(
                            CredentialResult.Err(
                                when (e) {
                                    is GetCredentialCancellationException ->
                                        err("canceled", e.message ?: "user canceled Google sign-in")
                                    is NoCredentialException ->
                                        err("unavailable", e.message ?: "No credentials available")
                                    else -> err("unavailable", e.message ?: e.toString())
                                },
                            ),
                        )
                        latch.countDown()
                    }
                },
            )
            if (!latch.await(TASK_TIMEOUT_SECONDS, TimeUnit.SECONDS)) {
                return CredentialResult.Err(err("native", "timeout waiting for Google credential"))
            }
            return result.get()
                ?: CredentialResult.Err(err("native", "Google credential result missing"))
        } finally {
            executor.shutdown()
        }
    }

    private fun parseGoogleCredentialResponse(response: GetCredentialResponse): CredentialResult {
        val credential = response.credential
        if (credential is CustomCredential &&
            credential.type == GoogleIdTokenCredential.TYPE_GOOGLE_ID_TOKEN_CREDENTIAL
        ) {
            return try {
                val googleId = GoogleIdTokenCredential.createFrom(credential.data)
                val token = googleId.idToken
                if (token.isBlank()) {
                    CredentialResult.Err(err("invalid-credential", "Google ID token empty"))
                } else {
                    CredentialResult.Ok(token)
                }
            } catch (t: Throwable) {
                CredentialResult.Err(err("invalid-credential", t.message ?: t.toString()))
            }
        }
        return CredentialResult.Err(err("unavailable", "unexpected credential type: ${credential.type}"))
    }

    // --- Helpers: Task & Threading ---

    private fun <T> awaitTask(block: () -> Task<T>, onSuccess: (T) -> String): String {
        if (Looper.myLooper() == Looper.getMainLooper()) {
            return err("native", "Firebase Auth waits must not run on the main looper; use spawn_blocking")
        }
        return try {
            onSuccess(Tasks.await(block(), TASK_TIMEOUT_SECONDS, TimeUnit.SECONDS))
        } catch (t: Throwable) {
            mapThrowable(t)
        }
    }

    private inline fun safeCall(block: () -> String): String = try {
        block()
    } catch (t: Throwable) {
        err("native", t.message ?: t.toString())
    }

    private fun <T> runOnMainSync(block: () -> T): T {
        if (Looper.myLooper() == Looper.getMainLooper()) return block()

        val latch = CountDownLatch(1)
        val result = AtomicReference<T>()
        val error = AtomicReference<Throwable>()

        Handler(Looper.getMainLooper()).post {
            try {
                result.set(block())
            } catch (t: Throwable) {
                error.set(t)
            } finally {
                latch.countDown()
            }
        }

        if (!latch.await(TASK_TIMEOUT_SECONDS, TimeUnit.SECONDS)) {
            throw IllegalStateException("timed out waiting for main looper")
        }
        error.get()?.let { throw it }
        @Suppress("UNCHECKED_CAST")
        return result.get() as T
    }

    // --- Helpers: Reply & Serialization Protocol ---

    private fun ok(): String = "OK"

    private fun okUser(user: FirebaseUser?): String =
        user?.let { "OK$UNIT${userFields(it)}" } ?: ok()

    /** Top-level Firebase fields + every `providerData` row (no preference merge). */
    private fun userFields(user: FirebaseUser): String {
        val providerData = user.providerData
        val sb = StringBuilder()
        sb.append(user.uid)
        sb.append(UNIT).append(user.email.orEmpty())
        sb.append(UNIT).append(user.displayName.orEmpty())
        sb.append(UNIT).append(user.photoUrl?.toString().orEmpty())
        sb.append(UNIT).append(providerData.size)
        for (info in providerData) {
            sb.append(UNIT).append(info.providerId.orEmpty())
            sb.append(UNIT).append(info.uid.orEmpty())
            sb.append(UNIT).append(info.email.orEmpty())
            sb.append(UNIT).append(info.displayName.orEmpty())
            sb.append(UNIT).append(info.photoUrl?.toString().orEmpty())
        }
        return sb.toString()
    }

    private fun updateProfile(displayName: String?, photoUrl: String?): String {
        val user = auth.currentUser ?: return noCurrentUser()
        val builder = UserProfileChangeRequest.Builder()
        if (displayName != null) {
            builder.setDisplayName(displayName)
        }
        if (photoUrl != null) {
            val trimmed = photoUrl.trim()
            if (trimmed.isEmpty()) {
                builder.setPhotoUri(null)
            } else {
                builder.setPhotoUri(Uri.parse(trimmed))
            }
        }
        val request = builder.build()
        return awaitTask({
            Tasks.await(user.updateProfile(request), TASK_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            user.reload()
        }) { ok() }
    }

    private fun noCurrentUser(): String =
        err("no-current-user", "No user currently signed in.")

    private fun err(code: String, message: String): String =
        "ERR$UNIT$code$UNIT$message"

    private fun mapThrowable(t: Throwable): String {
        var cur: Throwable? = t
        while (cur != null) {
            if (cur is FirebaseAuthException) {
                return err(toWireCode(cur.errorCode), cur.message ?: cur.errorCode)
            }
            // Do not substring-match "canceled"/"cancelled" in messages — that
            // mislabels Task/worker failures as user dismiss. Real sheet dismiss
            // is already `GetCredentialCancellationException` in getCredential.
            cur = cur.cause
        }
        return err("native", t.message ?: t.toString())
    }

    private fun toWireCode(androidCode: String): String {
        val normalized = androidCode.removePrefix("ERROR_").lowercase().replace('_', '-')
        return when (normalized) {
            "wrong-password", "user-not-found" -> "invalid-credential"
            else -> normalized.ifEmpty { "unknown" }
        }
    }
}
