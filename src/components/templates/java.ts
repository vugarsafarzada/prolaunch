import type { ProjectTemplate } from "./types";

export const javaTemplates: ProjectTemplate[] = [
  {
    cardId: "java-basic",
    title: "Java",
    framework: "Java",
    language: "Java",
    description: "Minimal Java app with a single Main.java entrypoint.",
    tags: ["Java", "CLI", "JDK"],
    versions: [
      {
        id: "java-basic-latest",
        label: "Latest",
        command: "javac Main.java && java Main",
      },
    ],
  },
  {
    cardId: "maven-java",
    title: "Maven",
    framework: "Maven",
    language: "Java",
    description: "Java application scaffold using Maven and exec plugin.",
    tags: ["Java", "Maven", "Build"],
    versions: [
      {
        id: "maven-java-latest",
        label: "Latest",
        command: "mvn exec:java",
      },
    ],
  },
  {
    cardId: "gradle-java",
    title: "Gradle",
    framework: "Gradle",
    language: "Java",
    description: "Java application scaffold using Gradle application plugin.",
    tags: ["Java", "Gradle", "Build"],
    versions: [
      {
        id: "gradle-java-latest",
        label: "Latest",
        command: "gradle run",
      },
    ],
  },
  {
    cardId: "spring-boot-java",
    title: "Spring Boot",
    framework: "Spring Boot",
    language: "Java",
    description: "Spring Boot web API starter using Maven.",
    tags: ["Java", "Spring Boot", "API"],
    versions: [
      {
        id: "spring-boot-java-latest",
        label: "3.5.14",
        command: "mvn spring-boot:run",
      },
    ],
  },
];
