import org.gradle.api.tasks.bundling.AbstractArchiveTask

plugins {
    id("com.android.library") version "8.4.2"
    kotlin("android") version "1.9.24"
}

android {
    namespace = "io.dioxus.firebase"
    compileSdk = 34

    defaultConfig {
        minSdk = 24
        lint.targetSdk = 34
        testOptions.targetSdk = 34
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
    }
}

dependencies {
    // Import the BoM for the Firebase platform
    implementation(platform("com.google.firebase:firebase-bom:34.19.0"))

    // Add the dependency for the Firebase Authentication library
    // When using the BoM, you don't specify versions in Firebase library dependencies
    implementation("com.google.firebase:firebase-auth")
}

tasks.withType<AbstractArchiveTask>().configureEach {
    archiveBaseName.set("dioxus-firebase-host")
}
