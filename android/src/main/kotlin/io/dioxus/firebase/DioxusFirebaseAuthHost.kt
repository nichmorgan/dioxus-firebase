package io.dioxus.firebase

import android.content.Context
import android.os.Handler
import android.os.Looper
import com.google.android.gms.tasks.Task
import com.google.android.gms.tasks.Tasks
import com.google.firebase.FirebaseApp
import com.google.firebase.FirebaseOptions
import com.google.firebase.auth.FirebaseAuth
import com.google.firebase.auth.FirebaseAuthException
import com.google.firebase.auth.FirebaseUser
import com.google.firebase.auth.UserProfileChangeRequest
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

/**
 * Thin Kotlin host so Rust can call Firebase Auth via JNI.
 *
 * Reply protocol (matches Rust `host_protocol`):
 * - success void: `OK`
 * - success user: `OK\u001fuid\u001femail\u001fdisplayName`
 * - success optional string: `OK` or `OK\u001fvalue`
 * - error: `ERR\u001f<code>\u001f<message>`
 */
object DioxusFirebaseAuthHost {
    private const val UNIT = '\u001f'
    private const val TASK_TIMEOUT_SECONDS = 60L
    private val subscribed = AtomicBoolean(false)

    private val auth: FirebaseAuth
        get() = FirebaseAuth.getInstance()

    // --- Public JNI API ---

    @JvmStatic
    fun initialize(
        context: Context,
        apiKey: String,
        appId: String,
        projectId: String,
        messagingSenderId: String,
        storageBucket: String,
        databaseUrl: String,
    ): String = safeCall {
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
    fun signInWithEmail(email: String, password: String): String =
        awaitTask({ auth.signInWithEmailAndPassword(email.trim(), password) }) { okUser(it.user) }

    @JvmStatic
    fun createUserWithEmail(email: String, password: String): String =
        awaitTask({ auth.createUserWithEmailAndPassword(email.trim(), password) }) { okUser(it.user) }

    @JvmStatic
    fun updateDisplayName(displayName: String): String {
        val user = auth.currentUser ?: return err("requires-recent-login", "no current user")
        val request = UserProfileChangeRequest.Builder().setDisplayName(displayName).build()
        return awaitTask({
            Tasks.await(user.updateProfile(request), TASK_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            user.reload()
        }) { ok() }
    }

    @JvmStatic
    fun sendPasswordResetEmail(email: String): String =
        awaitTask({ auth.sendPasswordResetEmail(email.trim()) }) { ok() }

    @JvmStatic
    fun signOut(): String = safeCall {
        auth.signOut()
        ok()
    }

    @JvmStatic
    fun currentUser(): String = safeCall {
        okUser(auth.currentUser)
    }

    @JvmStatic
    fun idToken(forceRefresh: Boolean): String {
        val user = auth.currentUser ?: return ok()
        return awaitTask({ user.getIdToken(forceRefresh) }) { result ->
            val token = result?.token
            if (token.isNullOrEmpty()) ok() else "OK$UNIT$token"
        }
    }

    @JvmStatic
    fun useEmulator(host: String, port: Int): String = safeCall {
        auth.useEmulator(host.trim(), port)
        ok()
    }

    @JvmStatic
    fun subscribeAuthState(): String {
        if (!subscribed.compareAndSet(false, true)) {
            onAuthStateChanged(userPayload(auth.currentUser))
            return ok()
        }
        return safeCall {
            auth.addAuthStateListener { a -> onAuthStateChanged(userPayload(a.currentUser)) }
            ok()
        }
    }

    /** Implemented in Rust via [JNIEnv.RegisterNatives]. */
    @JvmStatic
    external fun onAuthStateChanged(userPayload: String?)

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

    private fun userPayload(user: FirebaseUser?): String? =
        user?.let(::userFields)

    private fun userFields(user: FirebaseUser): String =
        "${user.uid}$UNIT${user.email.orEmpty()}$UNIT${user.displayName.orEmpty()}"

    private fun err(code: String, message: String): String =
        "ERR$UNIT$code$UNIT$message"

    private fun mapThrowable(t: Throwable): String {
        var cur: Throwable? = t
        while (cur != null) {
            if (cur is FirebaseAuthException) {
                return err(toWireCode(cur.errorCode), cur.message ?: cur.errorCode)
            }
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