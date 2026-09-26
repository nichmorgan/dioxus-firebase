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
    private static var authListenerHandle: AuthStateDidChangeListenerHandle?
    private static var rustCallback: (@convention(c) (UnsafePointer<CChar>?) -> Void)?

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
        return runOnMainSync {
            if FirebaseApp.app() == nil {
                let options = FirebaseOptions(googleAppID: appId.trimmingCharacters(in: .whitespacesAndNewlines),
                                              gcmSenderID: messagingSenderId.isEmpty ? "0" : messagingSenderId)
                options.apiKey = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
                options.projectID = projectId.trimmingCharacters(in: .whitespacesAndNewlines)
                if !storageBucket.isEmpty {
                    options.storageBucket = storageBucket
                }
                if !databaseURL.isEmpty {
                    options.databaseURL = databaseURL
                }
                FirebaseApp.configure(options: options)
            }
            _ = Auth.auth()
            return ok()
        }
    }

    @objc(signInWithEmail:password:)
    public static func signIn(email: String, password: String) -> String {
        return awaitAuth { completion in
            Auth.auth().signIn(withEmail: email.trimmingCharacters(in: .whitespacesAndNewlines),
                              password: password,
                              completion: completion)
        }
    }

    @objc(createUserWithEmail:password:)
    public static func createUser(email: String, password: String) -> String {
        return awaitAuth { completion in
            Auth.auth().createUser(withEmail: email.trimmingCharacters(in: .whitespacesAndNewlines),
                                   password: password,
                                   completion: completion)
        }
    }

    @objc(updateDisplayName:)
    public static func updateDisplayName(_ displayName: String) -> String {
        guard let user = Auth.auth().currentUser else {
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
            Auth.auth().sendPasswordReset(withEmail: email.trimmingCharacters(in: .whitespacesAndNewlines),
                                          completion: completion)
        }
    }

    @objc(signOut)
    public static func signOut() -> String {
        do {
            try Auth.auth().signOut()
            return ok()
        } catch {
            return mapError(error)
        }
    }

    @objc(currentUser)
    public static func currentUser() -> String {
        return okUser(Auth.auth().currentUser)
    }

    @objc(idTokenWithForceRefresh:)
    public static func idToken(forceRefresh: Bool) -> String {
        guard let user = Auth.auth().currentUser else {
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
        Auth.auth().useEmulator(withHost: host.trimmingCharacters(in: .whitespacesAndNewlines),
                                port: Int(port))
        return ok()
    }

    /// `callback` is a function pointer to `dioxus_firebase_auth_state_changed`.
    @objc(subscribeAuthStateWithCallback:)
    public static func subscribeAuthState(callback: UnsafeRawPointer) -> String {
        let fn = unsafeBitCast(callback, to: (@convention(c) (UnsafePointer<CChar>?) -> Void).self)
        rustCallback = fn
        if authListenerHandle == nil {
            authListenerHandle = Auth.auth().addStateDidChangeListener { _, user in
                notifyRust(user)
            }
        } else {
            notifyRust(Auth.auth().currentUser)
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
                    finish(okUser(result?.user ?? Auth.auth().currentUser))
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
        let ns = error as NSError
        if ns.domain == AuthErrorDomain {
            let code = authWireCode(ns.code)
            return err(code, ns.localizedDescription)
        }
        return err("native", ns.localizedDescription)
    }

    private static func authWireCode(_ code: Int) -> String {
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

    private static func runOnMainSync<T>(_ block: () -> T) -> T {
        if Thread.isMainThread {
            return block()
        }
        var result: T!
        DispatchQueue.main.sync {
            result = block()
        }
        return result
    }
}
