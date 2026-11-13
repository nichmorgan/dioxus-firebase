import Foundation
import UIKit
import AuthenticationServices
import CryptoKit
import FirebaseCore
import FirebaseAuth
import GoogleSignIn

/// Thin ObjC-visible host so Rust can call Firebase Auth via `objc`.
///
/// Dioxus CLI 0.7+ compiles this Swift package into the iOS app. The package
/// depends on FirebaseAuth, FirebaseCore, and GoogleSignIn. Apps do not copy
/// this file.
///
/// Reply protocol (matches Rust `host_protocol`):
/// - success void: `OK`
/// - success user: `OK\u{1f}uid\u{1f}email\u{1f}displayName\u{1f}providerIds`
/// - success optional string: `OK` or `OK\u{1f}value`
/// - error: `ERR\u{1f}<code>\u{1f}<message>` with Firebase Auth wire codes
///
/// `providerIds` is comma-separated (e.g. `password,google.com`).
@objc(DioxusFirebaseAuthHost)
public class DioxusFirebaseAuthHost: NSObject {
    private static let unit: Character = "\u{1f}"
    private static let waitTimeout: TimeInterval = 60
    private static let subscribeLock = NSLock()
    private static var authListenerHandle: AuthStateDidChangeListenerHandle?
    private static var rustCallback: (@convention(c) (UnsafePointer<CChar>?) -> Void)?
    private static var googleWebClientId: String = ""
    private static var appleCoordinator: AppleSignInCoordinator?

    /// Default Firebase app (re-read each time; do not cache).
    private static var app: FirebaseApp? { FirebaseApp.app() }

    /// Auth for the default app (re-read each time; do not cache).
    private static var auth: Auth { Auth.auth() }

    /// Initialize default `FirebaseApp` when missing, using app-supplied options.
    @objc(initializeWithApiKey:appId:projectId:messagingSenderId:storageBucket:databaseURL:googleWebClientId:)
    public static func initialize(
        apiKey: String,
        appId: String,
        projectId: String,
        messagingSenderId: String,
        storageBucket: String,
        databaseURL: String,
        googleWebClientId: String
    ) -> String {
        do {
            return try runOnMainSync {
                self.googleWebClientId = googleWebClientId.trimmingCharacters(in: .whitespacesAndNewlines)
                if app == nil {
                    let trimmedAppId = appId.trimmingCharacters(in: .whitespacesAndNewlines)
                    // FIRApp.configure raises NSException for a non-iOS GOOGLE_APP_ID.
                    // do/catch does not catch that, and the process aborts.
                    guard isIosAppId(trimmedAppId) else {
                        return err(
                            "invalid-app-id",
                            "GOOGLE_APP_ID must be version:number:ios:<hex>"
                        )
                    }
                    let trimmedSender = messagingSenderId.trimmingCharacters(in: .whitespacesAndNewlines)
                    let trimmedBucket = storageBucket.trimmingCharacters(in: .whitespacesAndNewlines)
                    let trimmedDatabaseURL = databaseURL.trimmingCharacters(in: .whitespacesAndNewlines)
                    let options = FirebaseOptions(
                        googleAppID: trimmedAppId,
                        gcmSenderID: trimmedSender.isEmpty ? "0" : trimmedSender
                    )
                    options.apiKey = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
                    options.projectID = projectId.trimmingCharacters(in: .whitespacesAndNewlines)
                    if !trimmedBucket.isEmpty {
                        options.storageBucket = trimmedBucket
                    }
                    if !trimmedDatabaseURL.isEmpty {
                        options.databaseURL = trimmedDatabaseURL
                    }
                    FirebaseApp.configure(options: options)
                }
                _ = auth // Ensure Auth is created against the configured app
                configureGoogleSignInIfNeeded()
                return ok()
            }
        } catch {
            return mapError(error)
        }
    }

    @objc(signInWithEmail:password:)
    public static func signIn(email: String, password: String) -> String {
        return awaitAuth { completion in
            auth.signIn(withEmail: email.trimmingCharacters(in: .whitespacesAndNewlines),
                        password: password,
                        completion: completion)
        }
    }

    @objc(createUserWithEmail:password:)
    public static func createUser(email: String, password: String) -> String {
        return awaitAuth { completion in
            auth.createUser(withEmail: email.trimmingCharacters(in: .whitespacesAndNewlines),
                            password: password,
                            completion: completion)
        }
    }

    @objc(updateDisplayName:)
    public static func updateDisplayName(_ displayName: String) -> String {
        guard let user = auth.currentUser else {
            return err("requires-recent-login", "no current user")
        }
        let change = user.createProfileChangeRequest()
        change.displayName = displayName
        // Profile writes do not refresh the cached user.
        return awaitVoid { completion in
            change.commitChanges { error in
                if let error = error {
                    completion(error)
                } else {
                    user.reload(completion: completion)
                }
            }
        }
    }

    @objc(sendPasswordResetEmail:)
    public static func sendPasswordResetEmail(_ email: String) -> String {
        return awaitVoid { completion in
            auth.sendPasswordReset(withEmail: email.trimmingCharacters(in: .whitespacesAndNewlines),
                                   completion: completion)
        }
    }

    @objc(signOut)
    public static func signOut() -> String {
        do {
            try auth.signOut()
            // Best-effort: clear Google Sign-In session so the next sheet is not sticky.
            GIDSignIn.sharedInstance.signOut()
            return ok()
        } catch {
            return mapError(error)
        }
    }

    @objc(currentUser)
    public static func currentUser() -> String {
        return okUser(auth.currentUser)
    }

    @objc(idTokenWithForceRefresh:)
    public static func idToken(forceRefresh: Bool) -> String {
        guard let user = auth.currentUser else {
            return ok()
        }
        return awaitReply { finish in
            user.getIDTokenForcingRefresh(forceRefresh) { token, error in
                if let error = error {
                    finish(mapError(error))
                } else if let token = token, !token.isEmpty {
                    finish("OK\(unit)\(token)")
                } else {
                    finish(ok())
                }
            }
        }
    }

    @objc(useEmulatorWithHost:port:)
    public static func useEmulator(host: String, port: UInt32) -> String {
        auth.useEmulator(withHost: host.trimmingCharacters(in: .whitespacesAndNewlines),
                         port: Int(port))
        return ok()
    }

    /// `callback` is a function pointer to `dioxus_firebase_auth_state_changed`.
    @objc(subscribeAuthStateWithCallback:)
    public static func subscribeAuthState(callback: UnsafeRawPointer) -> String {
        let fn = unsafeBitCast(callback, to: (@convention(c) (UnsafePointer<CChar>?) -> Void).self)
        let alreadySubscribed: Bool
        subscribeLock.lock()
        rustCallback = fn
        if authListenerHandle == nil {
            authListenerHandle = auth.addStateDidChangeListener { _, user in
                notifyRust(user)
            }
            alreadySubscribed = false
        } else {
            alreadySubscribed = true
        }
        subscribeLock.unlock()

        if alreadySubscribed {
            notifyRust(auth.currentUser)
        }
        return ok()
    }

    @objc(signInWithGoogle)
    public static func signInWithGoogle() -> String {
        signInOrLinkWithGoogle(link: false)
    }

    @objc(linkWithGoogle)
    public static func linkWithGoogle() -> String {
        signInOrLinkWithGoogle(link: true)
    }

    @objc(signInWithApple)
    public static func signInWithApple() -> String {
        signInOrLinkWithApple(link: false)
    }

    @objc(linkWithApple)
    public static func linkWithApple() -> String {
        signInOrLinkWithApple(link: true)
    }

    // MARK: - Social

    private static func configureGoogleSignInIfNeeded() {
        let clientId: String
        if !googleWebClientId.isEmpty {
            clientId = googleWebClientId
        } else if let plist = Bundle.main.object(forInfoDictionaryKey: "GIDClientID") as? String,
                  !plist.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            clientId = plist.trimmingCharacters(in: .whitespacesAndNewlines)
        } else {
            return
        }
        GIDSignIn.sharedInstance.configuration = GIDConfiguration(clientID: clientId)
    }

    private static func resolvedGoogleClientId() -> String? {
        if !googleWebClientId.isEmpty {
            return googleWebClientId
        }
        if let plist = Bundle.main.object(forInfoDictionaryKey: "GIDClientID") as? String {
            let trimmed = plist.trimmingCharacters(in: .whitespacesAndNewlines)
            if !trimmed.isEmpty {
                return trimmed
            }
        }
        return nil
    }

    private static func signInOrLinkWithGoogle(link: Bool) -> String {
        if Thread.isMainThread {
            return err("native", "Firebase Auth waits must not run on the main thread; use spawn_blocking")
        }
        guard resolvedGoogleClientId() != nil else {
            return err("unavailable", "google_web_client_id (or GIDClientID) is required for Google sign-in")
        }
        configureGoogleSignInIfNeeded()
        guard let presenter = presentationViewController() else {
            return err("unavailable", "no presentation view controller for Google sign-in")
        }

        return awaitReply { finish in
            DispatchQueue.main.async {
                GIDSignIn.sharedInstance.signIn(withPresenting: presenter) { result, error in
                    if let error = error as NSError? {
                        if error.code == GIDSignInError.canceled.rawValue {
                            finish(err("cancelled", error.localizedDescription))
                        } else {
                            finish(err("unavailable", error.localizedDescription))
                        }
                        return
                    }
                    guard let idToken = result?.user.idToken?.tokenString, !idToken.isEmpty else {
                        finish(err("invalid-credential", "Google ID token missing"))
                        return
                    }
                    let accessToken = result?.user.accessToken.tokenString
                    let credential = GoogleAuthProvider.credential(
                        withIDToken: idToken,
                        accessToken: accessToken
                    )
                    applyCredential(credential, link: link, finish: finish)
                }
            }
        }
    }

    private static func signInOrLinkWithApple(link: Bool) -> String {
        if Thread.isMainThread {
            return err("native", "Firebase Auth waits must not run on the main thread; use spawn_blocking")
        }
        return awaitReply { finish in
            DispatchQueue.main.async {
                guard let rawNonce = randomNonceString() else {
                    finish(err("native", "failed to generate Apple sign-in nonce"))
                    return
                }
                let hashed = sha256(rawNonce)
                let coordinator = AppleSignInCoordinator(rawNonce: rawNonce) { outcome in
                    appleCoordinator = nil
                    switch outcome {
                    case .cancelled:
                        finish(err("cancelled", "user cancelled Apple sign-in"))
                    case .failed(let message):
                        finish(err("unavailable", message))
                    case .success(let appleIdToken, let fullName):
                        let credential = OAuthProvider.appleCredential(
                            withIDToken: appleIdToken,
                            rawNonce: rawNonce,
                            fullName: fullName
                        )
                        applyCredential(credential, link: link, finish: finish)
                    }
                }
                appleCoordinator = coordinator
                coordinator.start(hashedNonce: hashed)
            }
        }
    }

    private static func applyCredential(
        _ credential: AuthCredential,
        link: Bool,
        finish: @escaping (String) -> Void
    ) {
        if link {
            guard let user = auth.currentUser else {
                finish(err("requires-recent-login", "no current user"))
                return
            }
            user.link(with: credential) { result, error in
                if let error = error {
                    finish(mapError(error))
                } else {
                    finish(okUser(result?.user ?? auth.currentUser))
                }
            }
        } else {
            auth.signIn(with: credential) { result, error in
                if let error = error {
                    finish(mapError(error))
                } else {
                    finish(okUser(result?.user ?? auth.currentUser))
                }
            }
        }
    }

    private static func presentationViewController() -> UIViewController? {
        let scenes = UIApplication.shared.connectedScenes
            .compactMap { $0 as? UIWindowScene }
        let window = scenes
            .flatMap { $0.windows }
            .first { $0.isKeyWindow }
            ?? scenes.flatMap { $0.windows }.first
        return window?.rootViewController?.topMostViewController()
    }

    private static func notifyRust(_ user: User?) {
        guard let rustCallback = rustCallback else { return }
        if let user = user {
            userFields(user).withCString { rustCallback($0) }
        } else {
            rustCallback(nil)
        }
    }

    private static func awaitAuth(
        _ body: (@escaping (AuthDataResult?, Error?) -> Void) -> Void
    ) -> String {
        return awaitReply { finish in
            body { result, error in
                if let error = error {
                    finish(mapError(error))
                } else {
                    finish(okUser(result?.user ?? auth.currentUser))
                }
            }
        }
    }

    private static func awaitVoid(
        _ body: (@escaping (Error?) -> Void) -> Void
    ) -> String {
        return awaitReply { finish in
            body { error in
                if let error = error {
                    finish(mapError(error))
                } else {
                    finish(ok())
                }
            }
        }
    }

    private static func awaitReply(_ body: (@escaping (String) -> Void) -> Void) -> String {
        if Thread.isMainThread {
            return err("native", "Firebase Auth waits must not run on the main thread; use spawn_blocking")
        }
        let semaphore = DispatchSemaphore(value: 0)
        var reply: String?
        body { value in
            reply = value
            semaphore.signal()
        }
        if semaphore.wait(timeout: .now() + waitTimeout) == .timedOut {
            return err("native", "timeout")
        }
        return reply ?? err("native", "host returned null")
    }

    private static func ok() -> String { "OK" }

    private static func userFields(_ user: User) -> String {
        let email = user.email ?? ""
        let name = user.displayName ?? ""
        var seen = Set<String>()
        var providers: [String] = []
        for info in user.providerData {
            let id = info.providerID
            if !id.isEmpty && id != "firebase" && seen.insert(id).inserted {
                providers.append(id)
            }
        }
        let providerIds = providers.joined(separator: ",")
        return "\(user.uid)\(unit)\(email)\(unit)\(name)\(unit)\(providerIds)"
    }

    private static func okUser(_ user: User?) -> String {
        guard let user = user else { return ok() }
        return "OK\(unit)\(userFields(user))"
    }

    private static func err(_ code: String, _ message: String) -> String {
        "ERR\(unit)\(code)\(unit)\(message)"
    }

    /// `version:number:ios:<hex>`, with no empty segment.
    private static func isIosAppId(_ appId: String) -> Bool {
        let parts = appId.split(separator: ":", omittingEmptySubsequences: false)
        guard parts.count == 4 else { return false }
        return !parts[0].isEmpty
            && !parts[1].isEmpty
            && parts[2] == "ios"
            && !parts[3].isEmpty
            && parts[3].allSatisfy(\.isHexDigit)
    }

    private static func mapError(_ error: Error) -> String {
        if let host = error as? HostError {
            switch host {
            case .mainThreadTimeout:
                return err("native", "timed out waiting for main thread")
            }
        }
        var cur: NSError? = error as NSError
        while let ns = cur {
            if ns.domain == AuthErrorDomain {
                let code = toWireCode(from: ns)
                let message = authErrorMessage(ns)
                return err(code, message)
            }
            cur = ns.userInfo[NSUnderlyingErrorKey] as? NSError
        }
        let ns = error as NSError
        return err("native", ns.localizedDescription)
    }

    /// Prefer the user-facing failure reason (e.g. weak-password detail).
    private static func authErrorMessage(_ ns: NSError) -> String {
        if let reason = ns.userInfo[NSLocalizedFailureReasonErrorKey] as? String,
           !reason.isEmpty {
            return reason
        }
        return ns.localizedDescription
    }

    /// Normalize cross-platform Auth error names like Android `toWireCode`.
    private static func toWireCode(from ns: NSError) -> String {
        if let name = ns.userInfo[AuthErrorUserInfoNameKey] as? String {
            return normalizeWireCode(name)
        }
        return authWireCodeFallback(ns.code)
    }

    private static func normalizeWireCode(_ raw: String) -> String {
        var normalized = raw
        if normalized.hasPrefix("ERROR_") {
            normalized = String(normalized.dropFirst("ERROR_".count))
        }
        normalized = normalized.lowercased().replacingOccurrences(of: "_", with: "-")
        switch normalized {
        case "wrong-password", "user-not-found":
            return "invalid-credential"
        case "":
            return "unknown"
        default:
            return normalized
        }
    }

    /// Fallback when `AuthErrorUserInfoNameKey` is absent.
    private static func authWireCodeFallback(_ code: Int) -> String {
        guard let authCode = AuthErrorCode(rawValue: code) else {
            return "unknown"
        }
        switch authCode {
        case .invalidCredential, .wrongPassword, .userNotFound:
            return "invalid-credential"
        case .emailAlreadyInUse:
            return "email-already-in-use"
        case .invalidEmail:
            return "invalid-email"
        case .weakPassword:
            return "weak-password"
        case .userDisabled:
            return "user-disabled"
        case .tooManyRequests:
            return "too-many-requests"
        case .networkError:
            return "network-request-failed"
        case .requiresRecentLogin:
            return "requires-recent-login"
        case .operationNotAllowed:
            return "operation-not-allowed"
        case .accountExistsWithDifferentCredential:
            return "account-exists-with-different-credential"
        case .credentialAlreadyInUse:
            return "credential-already-in-use"
        case .providerAlreadyLinked:
            return "provider-already-linked"
        default:
            return "unknown"
        }
    }

    private enum HostError: Error {
        case mainThreadTimeout
    }

    private static func runOnMainSync<T>(_ block: @escaping () throws -> T) throws -> T {
        if Thread.isMainThread {
            return try block()
        }
        let semaphore = DispatchSemaphore(value: 0)
        var result: Result<T, Error>?
        DispatchQueue.main.async {
            do {
                result = .success(try block())
            } catch {
                result = .failure(error)
            }
            semaphore.signal()
        }
        if semaphore.wait(timeout: .now() + waitTimeout) == .timedOut {
            throw HostError.mainThreadTimeout
        }
        switch result {
        case .success(let value):
            return value
        case .failure(let error):
            throw error
        case .none:
            throw HostError.mainThreadTimeout
        }
    }

    private static func randomNonceString(length: Int = 32) -> String? {
        guard length > 0 else { return nil }
        let charset: [Character] = Array("0123456789ABCDEFGHIJKLMNOPQRSTUVXYZabcdefghijklmnopqrstuvwxyz-._")
        var result = ""
        var remaining = length
        while remaining > 0 {
            var randoms = [UInt8](repeating: 0, count: 16)
            let status = SecRandomCopyBytes(kSecRandomDefault, randoms.count, &randoms)
            if status != errSecSuccess {
                return nil
            }
            for random in randoms {
                if remaining == 0 { break }
                if random < charset.count {
                    result.append(charset[Int(random)])
                    remaining -= 1
                }
            }
        }
        return result
    }

    private static func sha256(_ input: String) -> String {
        let data = Data(input.utf8)
        let hash = SHA256.hash(data: data)
        return hash.compactMap { String(format: "%02x", $0) }.joined()
    }
}

// MARK: - Apple Sign In coordinator

private final class AppleSignInCoordinator: NSObject, ASAuthorizationControllerDelegate,
    ASAuthorizationControllerPresentationContextProviding
{
    enum Outcome {
        case success(idToken: String, fullName: PersonNameComponents?)
        case cancelled
        case failed(String)
    }

    private let rawNonce: String
    private let completion: (Outcome) -> Void

    init(rawNonce: String, completion: @escaping (Outcome) -> Void) {
        self.rawNonce = rawNonce
        self.completion = completion
    }

    func start(hashedNonce: String) {
        let provider = ASAuthorizationAppleIDProvider()
        let request = provider.createRequest()
        request.requestedScopes = [.fullName, .email]
        request.nonce = hashedNonce
        let controller = ASAuthorizationController(authorizationRequests: [request])
        controller.delegate = self
        controller.presentationContextProvider = self
        controller.performRequests()
    }

    func presentationAnchor(for controller: ASAuthorizationController) -> ASPresentationAnchor {
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        if let key = scenes.flatMap({ $0.windows }).first(where: { $0.isKeyWindow }) {
            return key
        }
        return scenes.flatMap({ $0.windows }).first ?? ASPresentationAnchor()
    }

    func authorizationController(
        controller: ASAuthorizationController,
        didCompleteWithAuthorization authorization: ASAuthorization
    ) {
        guard let credential = authorization.credential as? ASAuthorizationAppleIDCredential else {
            completion(.failed("unexpected Apple credential type"))
            return
        }
        guard let tokenData = credential.identityToken,
              let idToken = String(data: tokenData, encoding: .utf8),
              !idToken.isEmpty
        else {
            completion(.failed("Apple identity token missing"))
            return
        }
        completion(.success(idToken: idToken, fullName: credential.fullName))
    }

    func authorizationController(
        controller: ASAuthorizationController,
        didCompleteWithError error: Error
    ) {
        let ns = error as NSError
        if ns.code == ASAuthorizationError.canceled.rawValue {
            completion(.cancelled)
        } else {
            completion(.failed(ns.localizedDescription))
        }
    }
}

private extension UIViewController {
    func topMostViewController() -> UIViewController {
        if let presented = presentedViewController {
            return presented.topMostViewController()
        }
        if let nav = self as? UINavigationController, let visible = nav.visibleViewController {
            return visible.topMostViewController()
        }
        if let tab = self as? UITabBarController, let selected = tab.selectedViewController {
            return selected.topMostViewController()
        }
        return self
    }
}
