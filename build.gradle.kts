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

dependencies {
    implementation("commons-io:commons-io:2.14.0")
    implementation("org.apache.commons:commons-lang3:3.18.0")
    implementation("org.fusesource.jansi:jansi:2.4.0")
    implementation("org.apache.logging.log4j:log4j-api:2.24.3")
    implementation("org.apache.logging.log4j:log4j-core:2.25.3")
    implementation("org.apache.logging.log4j:log4j-1.2-api:2.24.3")
    implementation(files("libs/ecj-4.9.jar"))
    implementation(files("libs/nproperty-1.0.jar"))
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

tasks.jar {
    archiveFileName.set("l2-editor.jar")
    destinationDirectory.set(layout.buildDirectory.dir("../dist/lib"))
    manifest {
        attributes["Main-Class"] = "com.majestic.studio.Boot"
        attributes["Created-By"] = "${System.getProperty("java.version")} (${System.getProperty("java.vendor")})"
        attributes["Build-By"] = System.getProperty("user.name")
        attributes["Build-Date"] = SimpleDateFormat("yyyy.MM.dd HH:mm").format(Date())
        attributes["Implementation-Version"] = "1.3"
    }
    from(configurations.runtimeClasspath.get().map { if (it.isDirectory) it else zipTree(it) })
    duplicatesStrategy = DuplicatesStrategy.EXCLUDE
}