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
        targetSdk = 34
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
    implementation(platform("com.google.firebase:firebase-bom:33.7.0"))
    implementation("com.google.firebase:firebase-auth")
}

tasks.withType<AbstractArchiveTask>().configureEach {
    archiveBaseName.set("dioxus-firebase-host")
}
