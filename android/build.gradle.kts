import org.gradle.api.tasks.bundling.AbstractArchiveTask

plugins {
    id("com.android.library") version "8.7.3"
    kotlin("android") version "2.1.0"
}

android {
    namespace = "io.dioxus.firebase"
    compileSdk = 35

    defaultConfig {
        minSdk = 24
        lint.targetSdk = 35
        testOptions.targetSdk = 35
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        getByName("release") {
            isMinifyEnabled = false
        }
        getByName("debug") {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
        freeCompilerArgs = freeCompilerArgs + listOf("-Xskip-metadata-version-check")
    }
}

dependencies {
    implementation(platform("com.google.firebase:firebase-bom:34.19.0"))
    implementation("com.google.firebase:firebase-auth")
}

tasks.withType<AbstractArchiveTask>().configureEach {
    archiveBaseName.set("dioxus-firebase-host")
}

// dx writes an app network-security config that only allows cleartext to 127.0.0.1.
// That resource wins over any library copy, so debug builds of a consuming app
// (the :app module dx generates) must be patched here. 10.0.2.2 is the emulator's
// alias for the host machine; release builds are left unchanged.
gradle.projectsEvaluated {
    val app = rootProject.findProject(":app") ?: return@projectsEvaluated
    val hook =
        app.tasks.findByName("preDebugBuild") ?: app.tasks.findByName("preBuild") ?: return@projectsEvaluated
    // Lifecycle tasks are often up-to-date and then skip actions. This patch has to
    // run or the packaged config still blocks 10.0.2.2.
    hook.outputs.upToDateWhen { false }
    hook.doFirst {
        allowEmulatorCleartext(app)
    }
}

fun allowEmulatorCleartext(app: Project) {
    val config = app.layout.projectDirectory.file("src/main/res/xml/network_security_config.xml").asFile
    if (!config.isFile) {
        return
    }
    val text = config.readText()
    if (text.contains("10.0.2.2")) {
        return
    }
    val anchor = "<domain includeSubdomains=\"true\">127.0.0.1</domain>"
    val domains = buildString {
        append("        <domain includeSubdomains=\"true\">10.0.2.2</domain>")
        if (!text.contains(">localhost<")) {
            append("\n        <domain includeSubdomains=\"true\">localhost</domain>")
        }
    }
    val updated =
        when {
            text.contains(anchor) -> text.replace(anchor, "$anchor\n$domains")
            text.contains("</domain-config>") ->
                text.replace("</domain-config>", "$domains\n    </domain-config>")
            else -> return
        }
    config.writeText(updated)
    logger.lifecycle("Allowed cleartext HTTP to the Android emulator host (10.0.2.2)")
}
