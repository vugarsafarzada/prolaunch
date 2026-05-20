import type { ProjectTemplate } from "./types";

export const rubyTemplates: ProjectTemplate[] = [
  {
    cardId: "ruby-basic",
    title: "Ruby",
    framework: "Ruby",
    language: "Ruby",
    description: "Minimal Ruby app with a single main.rb entrypoint.",
    tags: ["Ruby", "CLI"],
    versions: [
      {
        id: "ruby-basic-latest",
        label: "Latest",
        command: "ruby main.rb",
      },
    ],
  },
  {
    cardId: "sinatra-ruby",
    title: "Sinatra",
    framework: "Sinatra",
    language: "Ruby",
    description: "Lightweight Ruby web app with Sinatra and Bundler.",
    tags: ["Ruby", "Sinatra", "Web"],
    versions: [
      {
        id: "sinatra-ruby-latest",
        label: "Latest",
        command: "bundle exec ruby app.rb",
      },
    ],
  },
  {
    cardId: "rails-ruby",
    title: "Rails",
    framework: "Rails",
    language: "Ruby",
    description: "Minimal Rails API starter with local Bundler install.",
    tags: ["Ruby", "Rails", "MVC"],
    versions: [
      {
        id: "rails-ruby-latest",
        label: "6.1.x",
        command: "bundle exec ruby -rlogger bin/rails server",
      },
    ],
  },
];
