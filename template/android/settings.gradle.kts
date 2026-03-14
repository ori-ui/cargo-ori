pluginManagement {
    repositories {
        gradlePluginPortal()
        google()
        mavenCentral()
        maven {
            url = uri("https://raw.githubusercontent.com/ori-ui/gradle-plugin/maven/")
        }
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
        maven {
            url = uri("https://raw.githubusercontent.com/ori-ui/gradle-plugin/maven/")
        }
    }
}
