import java.text.SimpleDateFormat
import java.util.Date

plugins {
    id("java")
}

group = "com.majestic.studio"
version = "1.0"

repositories {
    mavenCentral()
}

tasks.jar {
    archiveFileName.set("l2-editor.jar")
    destinationDirectory.set(layout.buildDirectory.dir("../dist/lib"))
    manifest{
        attributes["Created-By"] = "${System.getProperty("java.version")} (${System.getProperty("java.vendor")})"
        attributes["Build-By"] = System.getProperty("user.name")
        attributes["Build-Date"] = SimpleDateFormat("yyyy.MM.dd HH:mm").format(Date())
        attributes["Implementation-Version"] = "1.3"
    }
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

dependencies{
    "implementation"(fileTree("${rootProject.projectDir}/dist/lib") {
        include("*.jar")
        exclude("l2-editor.jar")
    })
}