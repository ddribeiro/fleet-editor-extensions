plugins {
    id("java")
    id("org.jetbrains.intellij") version "1.17.2"
}

group = "com.fleetdm"
version = "0.1.1"

repositories {
    mavenCentral()
}

intellij {
    version.set("2024.1")
    type.set("IC") // IntelliJ Community — works in GoLand, WebStorm, etc.
    plugins.set(listOf("org.jetbrains.plugins.yaml"))
}

tasks {
    patchPluginXml {
        sinceBuild.set("241")
        untilBuild.set("251.*")
    }

    buildSearchableOptions {
        enabled = false
    }
}
