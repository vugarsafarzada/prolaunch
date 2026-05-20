// Template icon assets, metadata map and the template-to-icon dispatcher.

import angularIcon from "../../../src-tauri/icons/supports/angular_logo.png";
import chiIcon from "../../../src-tauri/icons/supports/chi_logo.png";
import codeigniterIcon from "../../../src-tauri/icons/supports/codeIgniter_logo.png";
import dartIcon from "../../../src-tauri/icons/supports/dart_logo.png";
import djangoIcon from "../../../src-tauri/icons/supports/django_logo.png";
import echoIcon from "../../../src-tauri/icons/supports/echo_logo.png";
import expressIcon from "../../../src-tauri/icons/supports/express_logo.png";
import fastapiIcon from "../../../src-tauri/icons/supports/fastapi_logo.png";
import fiberIcon from "../../../src-tauri/icons/supports/fiber_logo.png";
import flaskIcon from "../../../src-tauri/icons/supports/flask_logo.png";
import flutterIcon from "../../../src-tauri/icons/supports/flutter_logo.png";
import ginIcon from "../../../src-tauri/icons/supports/gin_logo.png";
import goIcon from "../../../src-tauri/icons/supports/go_logo.png";
import gradleIcon from "../../../src-tauri/icons/supports/gradle_logo.png";
import javaIcon from "../../../src-tauri/icons/supports/java_logo.png";
import javascriptIcon from "../../../src-tauri/icons/supports/javascript_logo.png";
import laravelIcon from "../../../src-tauri/icons/supports/laravel_logo.png";
import mavenIcon from "../../../src-tauri/icons/supports/maven_logo.png";
import nestjsIcon from "../../../src-tauri/icons/supports/nestjs_logo.png";
import nextIcon from "../../../src-tauri/icons/supports/nextjs_logo.png";
import nodeIcon from "../../../src-tauri/icons/supports/nodejs_logo.png";
import nuxtIcon from "../../../src-tauri/icons/supports/nuxtjs_logo.png";
import phpIcon from "../../../src-tauri/icons/supports/php_logo.png";
import pythonIcon from "../../../src-tauri/icons/supports/python_logo.png";
import reactIcon from "../../../src-tauri/icons/supports/react_logo.png";
import railsIcon from "../../../src-tauri/icons/supports/rails_logo.png";
import rubyIcon from "../../../src-tauri/icons/supports/ruby_logo.png";
import sinatraIcon from "../../../src-tauri/icons/supports/sinatra_logo.jpg";
import slimIcon from "../../../src-tauri/icons/supports/slim_logo.png";
import svelteIcon from "../../../src-tauri/icons/supports/svelte_logo.png";
import springBootIcon from "../../../src-tauri/icons/supports/spring_boot_logo.png";
import symfonyIcon from "../../../src-tauri/icons/supports/symfony_logo.png";
import typescriptIcon from "../../../src-tauri/icons/supports/typescript_logo.png";
import vueIcon from "../../../src-tauri/icons/supports/vuejs_logo.png";
import type { ProjectTemplate, TemplateIconKey, TemplateIconMeta } from "./types";

export const TEMPLATE_ICONS: Record<TemplateIconKey, TemplateIconMeta> = {
  angular: { label: "A", title: "Angular", className: "angular", src: angularIcon },
  chi: { label: "Ch", title: "Chi", className: "chi", src: chiIcon },
  codeigniter: { label: "CI", title: "CodeIgniter", className: "codeigniter", src: codeigniterIcon },
  dart: { label: "D", title: "Dart", className: "dart", src: dartIcon },
  django: { label: "Dj", title: "Django", className: "django", src: djangoIcon },
  echo: { label: "Ec", title: "Echo", className: "echo", src: echoIcon },
  express: { label: "Ex", title: "Express", className: "express", src: expressIcon },
  fastapi: { label: "FA", title: "FastAPI", className: "fastapi", src: fastapiIcon },
  fiber: { label: "Fi", title: "Fiber", className: "fiber", src: fiberIcon },
  flutter: { label: "Fl", title: "Flutter", className: "flutter", src: flutterIcon },
  flask: { label: "Fl", title: "Flask", className: "flask", src: flaskIcon },
  gin: { label: "Gi", title: "Gin", className: "gin", src: ginIcon },
  go: { label: "Go", title: "Go", className: "go", src: goIcon },
  gradle: { label: "Gr", title: "Gradle", className: "gradle", src: gradleIcon },
  java: { label: "Ja", title: "Java", className: "java", src: javaIcon },
  javascript: { label: "JS", title: "JavaScript", className: "javascript", src: javascriptIcon },
  laravel: { label: "L", title: "Laravel", className: "laravel", src: laravelIcon },
  maven: { label: "Mv", title: "Maven", className: "maven", src: mavenIcon },
  nestjs: { label: "Ne", title: "NestJS", className: "nestjs", src: nestjsIcon },
  next: { label: "N", title: "Next.js", className: "next", src: nextIcon },
  node: { label: "N", title: "Node.js", className: "node", src: nodeIcon },
  nuxt: { label: "N", title: "Nuxt", className: "nuxt", src: nuxtIcon },
  php: { label: "PHP", title: "PHP", className: "php", src: phpIcon },
  python: { label: "Py", title: "Python", className: "python", src: pythonIcon },
  rails: { label: "Ra", title: "Rails", className: "rails", src: railsIcon },
  react: { label: "R", title: "React", className: "react", src: reactIcon },
  reactNative: { label: "RN", title: "React Native", className: "react-native", src: reactIcon },
  ruby: { label: "Rb", title: "Ruby", className: "ruby", src: rubyIcon },
  sinatra: { label: "Si", title: "Sinatra", className: "sinatra", src: sinatraIcon },
  slim: { label: "S", title: "Slim", className: "slim", src: slimIcon },
  svelte: { label: "S", title: "Svelte", className: "svelte", src: svelteIcon },
  springBoot: { label: "SB", title: "Spring Boot", className: "spring-boot", src: springBootIcon },
  symfony: { label: "Sf", title: "Symfony", className: "symfony", src: symfonyIcon },
  typescript: { label: "TS", title: "TypeScript", className: "typescript", src: typescriptIcon },
  vue: { label: "V", title: "Vue", className: "vue", src: vueIcon },
};

export function templateIconKey(template: ProjectTemplate): TemplateIconKey {
  if (template.cardId.includes("rails")) return "rails";
  if (template.cardId.includes("sinatra")) return "sinatra";
  if (template.language === "Ruby") return "ruby";
  if (template.cardId.includes("spring-boot")) return "springBoot";
  if (template.cardId.includes("maven")) return "maven";
  if (template.cardId.includes("gradle")) return "gradle";
  if (template.language === "Java") return "java";
  if (template.cardId.includes("gin")) return "gin";
  if (template.cardId.includes("fiber")) return "fiber";
  if (template.cardId.includes("echo")) return "echo";
  if (template.cardId.includes("chi")) return "chi";
  if (template.language === "Go") return "go";
  if (template.cardId.includes("flutter")) return "flutter";
  if (template.cardId.includes("dart")) return "dart";
  if (template.cardId.includes("fastapi")) return "fastapi";
  if (template.cardId.includes("flask")) return "flask";
  if (template.cardId.includes("django")) return "django";
  if (template.cardId.includes("node")) return "node";
  if (template.cardId.includes("express")) return "express";
  if (template.cardId.includes("nestjs")) return "nestjs";
  if (template.cardId.includes("react-native")) return "reactNative";
  if (template.cardId.includes("react") || template.cardId.includes("cra")) return "react";
  if (template.cardId.includes("next")) return "next";
  if (template.cardId.includes("vue")) return "vue";
  if (template.cardId.includes("nuxt")) return "nuxt";
  if (template.cardId.includes("svelte")) return "svelte";
  if (template.cardId.includes("laravel")) return "laravel";
  if (template.cardId.includes("symfony")) return "symfony";
  if (template.cardId.includes("slim")) return "slim";
  if (template.cardId.includes("codeigniter")) return "codeigniter";
  if (template.cardId.includes("angular")) return "angular";
  if (template.language === "Python") return "python";
  if (template.language === "PHP") return "php";
  if (template.language === "TypeScript") return "typescript";
  return "javascript";
}
