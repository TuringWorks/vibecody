---
triggers: ["Objective-C", "ObjC", "Objective C", "NSObject", "Foundation framework", "UIKit Objective-C", "Cocoa Objective-C", "ARC Objective-C", "@interface", "@implementation"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["clang"]
category: objective-c
---

# Objective-C

When writing Objective-C code (iOS/macOS legacy codebases):

1. Use ARC (Automatic Reference Counting) — never call `retain`/`release`/`autorelease` manually; use `strong` for owning references, `weak` for non-owning (delegates, parent pointers), `copy` for value-type properties (NSString, NSArray, blocks).
2. Define classes with `@interface`/`@implementation`: `@interface Person : NSObject @property (nonatomic, copy) NSString *name; - (void)greet; @end` — use class extensions `@interface Person ()` in `.m` files for private properties/methods.
3. Use modern Objective-C syntax: `@[@"a", @"b"]` for array literals, `@{@"key": @"value"}` for dictionaries, `@42` for number literals, `array[0]` for subscripting — these are cleaner than `[NSArray arrayWithObjects:...]`.
4. Use blocks (closures) for callbacks: `void (^completion)(BOOL success) = ^(BOOL success) { NSLog(@"Done: %d", success); };` — use `__weak typeof(self) weakSelf = self;` in blocks that capture `self` to prevent retain cycles.
5. Follow Cocoa naming conventions: init methods start with `init` (`initWithName:`), factory methods with class name (`personWithName:`), getters use bare property name (`name` not `getName`), boolean properties use `is` prefix (`isActive`).
6. Use `NSError **` for error handling: `- (BOOL)saveData:(NSError **)error { if (failed) { *error = [NSError errorWithDomain:@"MyApp" code:100 userInfo:@{NSLocalizedDescriptionKey: @"Save failed"}]; return NO; } return YES; }`.
7. Use GCD (Grand Central Dispatch) for concurrency: `dispatch_async(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^{ ... dispatch_async(dispatch_get_main_queue(), ^{ [self updateUI]; }); });` — never block the main thread.
8. Use protocols for interfaces: `@protocol DataSource <NSObject> - (NSInteger)numberOfItems; @optional - (NSString *)titleForItem:(NSInteger)index; @end` — `@optional` methods must be checked with `respondsToSelector:` before calling.
9. Use categories for extending classes: `@interface NSString (Validation) - (BOOL)isValidEmail; @end` — categories add methods to existing classes without subclassing; use class extensions for private interface in implementation files.
10. For interop with Swift: use bridging headers (`ProjectName-Bridging-Header.h`) for Swift-to-ObjC; use `NS_SWIFT_NAME()` and `NS_REFINED_FOR_SWIFT` annotations for cleaner Swift API; mark nullable/nonnull with `_Nullable`/`_Nonnull`.
11. Use `NSPredicate` for filtering: `[array filteredArrayUsingPredicate:[NSPredicate predicateWithFormat:@"age > %d", 18]]` — predicates work with Core Data, collections, and KVC; support compound predicates with `AND`/`OR`.
12. Test with XCTest: `- (void)testAddition { XCTAssertEqual([calc add:2 to:2], 4, @"2+2 should be 4"); }` — use `XCTestExpectation` for async tests; `OCMock` for mocking.
