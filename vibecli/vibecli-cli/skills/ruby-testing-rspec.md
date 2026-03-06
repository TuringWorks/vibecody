---
triggers: ["RSpec", "FactoryBot", "Capybara", "ruby test", "rspec describe", "shoulda matchers"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["ruby"]
category: testing
---

# Ruby Testing with RSpec

When testing Ruby applications with RSpec:

1. Structure: `describe` for class/method, `context` for conditions, `it` for behaviors
2. Use `let` for lazy-evaluated test data; `let!` for eager evaluation (needed for DB records)
3. Use FactoryBot: `create(:user)` for persisted, `build(:user)` for in-memory, `build_stubbed` for fastest
4. Use `subject { described_class.new(params) }` for the object under test
5. Use `expect(result).to eq(expected)` — one expectation per example when possible
6. Use `before { }` for setup, `after { }` for teardown — prefer `let` over `before` for data
7. Use `shared_examples` for behavior shared across multiple specs
8. Capybara for integration tests: `visit path`, `fill_in 'Email'`, `click_button 'Submit'`, `expect(page).to have_content`
9. Use `have_http_status(:ok)` for controller specs; `be_valid` for model specs
10. Mock with `allow(obj).to receive(:method).and_return(value)` — verify with `expect().to have_received`
11. Use `shoulda-matchers` for one-liner model validations: `it { should validate_presence_of(:name) }`
12. Use `DatabaseCleaner` with transaction strategy — fast rollback between tests
