---
triggers: ["Ruby on Rails", "ActiveRecord", "rails migration", "rails model", "rails controller", "devise", "rails routes"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["ruby"]
category: ruby
---

# Ruby on Rails

When building Rails applications:

1. Follow Rails conventions: model names singular (`User`), table names plural (`users`)
2. Use `rails generate model/controller/migration` — don't create files manually
3. Migrations: `add_column`, `create_table`, `add_index` — always add indexes for foreign keys
4. Use `has_many`, `belongs_to`, `has_many :through` for associations — let ActiveRecord handle joins
5. Use scopes for reusable queries: `scope :active, -> { where(active: true) }`
6. Use `strong_parameters`: `params.require(:user).permit(:name, :email)` in controllers
7. Use `before_action` for shared controller logic (auth, loading resources)
8. Validation in models: `validates :email, presence: true, uniqueness: true, format: { with: URI::MailTo::EMAIL_REGEXP }`
9. Use `N+1` detection with `bullet` gem — always `includes()` associated records
10. Background jobs with `ActiveJob` + Sidekiq/GoodJob — never do heavy work in request cycle
11. Use `rails routes` to inspect all routes; RESTful resources with `resources :users`
12. Test with `Minitest` (Rails default) or RSpec — use `FactoryBot` for test data
