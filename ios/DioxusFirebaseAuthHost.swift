import Foundation
import FirebaseCore
import FirebaseAuth

/// Thin ObjC-visible host so Rust can call Firebase Auth via `objc`.
///
/// Compile this file into your Dioxus iOS target and add Firebase iOS SDK
/// (SPM: https://github.com/firebase/firebase-ios-sdk — FirebaseAuth).
///
/// Reply protocol (matches Rust `host_protocol`):
/// - success void: `OK`
/// - success user: `OK\u{1f}uid\u{1f}email\u{1f}displayName`
/// - success optional string: `OK` or `OK\u{1f}value`
/// - error: `ERR\u{1f}<code>\u{1f}<message>` with Firebase Auth wire codes
@objc(DioxusFirebaseAuthHost)
public class DioxusFirebaseAuthHost: NSObject {
    private static let unit: Character = "\u{1f}"
    private static let waitTimeout: TimeInterval = 60
    private static let subscribeLock = NSLock()
    private static var authListenerHandle: AuthStateDidChangeListenerHandle?
    private static var rustCallback: (@convention(c) (UnsafePointer<CChar>?) -> Void)?

    /// Default Firebase app (re-read each time; do not cache).
    private static var app: FirebaseApp? { FirebaseApp.app() }

    /// Auth for the default app (re-read each time; do not cache).
    private static var auth: Auth { Auth.auth() }

    /// Initialize default `FirebaseApp` when missing, using app-supplied options.
    @objc(initializeWithApiKey:appId:projectId:messagingSenderId:storageBucket:databaseURL:)
    public static func initialize(
        apiKey: String,
        appId: String,
        projectId: String,
        messagingSenderId: String,
        storageBucket: String,
        databaseURL: String
    ) -> String {
        do {
            return try runOnMainSync {
                if app == nil {
                    let trimmedSender = messagingSenderId.trimmingCharacters(in: .whitespacesAndNewlines)
                    let trimmedBucket = storageBucket.trimmingCharacters(in: .whitespacesAndNewlines)
                    let trimmedDatabaseURL = databaseURL.trimmingCharacters(in: .whitespacesAndNewlines)
                    let options = FirebaseOptions(
                        googleAppID: appId.trimmingCharacters(in: .whitespacesAndNewlines),
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
        return "\(user.uid)\(unit)\(email)\(unit)\(name)"
    }

    private static func okUser(_ user: User?) -> String {
        guard let user = user else { return ok() }
        return "OK\(unit)\(userFields(user))"
    }

    private static func err(_ code: String, _ message: String) -> String {
        "ERR\(unit)\(code)\(unit)\(message)"
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
        guard let authCode = AuthErrorCode.Code(rawValue: code) else {
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
        default:
            return "unknown"
        }
    }

    private enum HostError: Error {
        case mainThreadTimeout
    }

    private static func runOnMainSync<T>(_ block: () throws -> T) throws -> T {
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
}
