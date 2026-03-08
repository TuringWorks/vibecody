---
triggers: ["Perl", "perl script", "CPAN", "regex Perl", "Perl one-liner", "Moose", "Mojo", "Mojolicious", "perl module"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["perl"]
category: perl
---

# Perl

When writing Perl code:

1. Always start scripts with `use strict; use warnings;` — `strict` forces variable declaration and prevents symbolic references; `warnings` catches common mistakes like uninitialized variables and deprecated syntax.
2. Use `my` for lexical scoping: `my $count = 0; my @items = (1, 2, 3); my %lookup = (key => 'value');` — avoid package globals; use `our` only for intentional package-level variables.
3. Perl's regex is best-in-class: `if ($str =~ m/^(\d{4})-(\d{2})-(\d{2})$/) { my ($year, $month, $day) = ($1, $2, $3); }` — use named captures `(?<year>\d{4})` for readability; use `qr//` for precompiled patterns.
4. Use `Moose` or `Moo` for OOP: `package Animal; use Moose; has 'name' => (is => 'ro', isa => 'Str', required => 1); sub speak { ... }` — Moose provides types, roles, method modifiers; Moo is the lightweight alternative.
5. Handle files safely: `open(my $fh, '<:encoding(UTF-8)', $filename) or die "Cannot open $filename: $!";` — use 3-argument open, lexical filehandles, and encoding layers; use `Path::Tiny` for modern file operations.
6. Use CPAN modules: `cpanm Module::Name` to install; `use JSON::MaybeXS; use LWP::UserAgent; use DBI; use DateTime;` — CPAN has 200k+ modules; prefer well-maintained modules with good test coverage.
7. For web development: use Mojolicious (`Mojo::Base`) for full-stack or Dancer2 for lightweight apps — `get '/' => sub { my $c = shift; $c->render(text => 'Hello') };` — both support WebSocket, async, and templates.
8. Database access with DBI: `my $dbh = DBI->connect($dsn, $user, $pass, {RaiseError => 1}); my $sth = $dbh->prepare('SELECT * FROM users WHERE id = ?'); $sth->execute($id);` — always use placeholders.
9. Use `map`, `grep`, and `sort` for data transformation: `my @names = map { $_->{name} } grep { $_->{active} } @users;` — functional style is idiomatic Perl; use `List::Util` for `reduce`, `min`, `max`, `sum`.
10. Write tests with `Test::More`: `use Test::More tests => 3; ok($result, 'got result'); is($count, 42, 'correct count'); like($msg, qr/success/, 'message matches');` — run with `prove -l t/`.
11. One-liners for text processing: `perl -ne 'print if /pattern/' file.txt` (grep), `perl -pe 's/old/new/g' file.txt` (sed), `perl -ane 'print $F[2]\n' file.txt` (awk) — Perl excels at command-line text manipulation.
12. Use `Try::Tiny` for exception handling: `try { risky_operation() } catch { warn "Failed: $_" };` — avoids pitfalls of `eval { }; if ($@) { }` where `$@` can be clobbered.
