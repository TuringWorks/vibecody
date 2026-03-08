---
triggers: ["ActionScript", "ActionScript 3", "AS3", "Flash", "Adobe AIR", "SWF", "Flex", "Flash migration", "Apache Royale"]
tools_allowed: ["read_file", "write_file", "bash"]
category: legacy
---

# ActionScript

When maintaining or migrating ActionScript/Flash codebases:

1. ActionScript 3 (AS3) is effectively dead (Flash Player EOL Dec 2020) — prioritize migration to HTML5/JavaScript/TypeScript; use AS3 knowledge for understanding legacy codebases and planning migration strategies.
2. AS3 was a typed ECMAScript: `package com.example { public class Player extends Sprite { private var _score:int = 0; public function get score():int { return _score; } } }` — similar to TypeScript with classes, interfaces, and static typing.
3. Display list architecture: `Stage` → `DisplayObjectContainer` → `Sprite`/`MovieClip` — `addChild(sprite)` to display; `removeChild(sprite)` to hide; `addEventListener(Event.ENTER_FRAME, update)` for game loops at the frame rate.
4. Event model: `addEventListener(MouseEvent.CLICK, onClick)` — bubbling and capture phases; `removeEventListener` to prevent memory leaks; custom events: `dispatchEvent(new CustomEvent("dataLoaded", data))`.
5. Migration to HTML5 Canvas/WebGL: `Sprite` → HTML5 Canvas with PixiJS or Phaser; `MovieClip` → sprite sheet animations; `TextField` → DOM elements or canvas text; `Sound` → Web Audio API; `URLLoader` → `fetch()`.
6. Migration to TypeScript: AS3 classes map nearly 1:1 to TypeScript classes; `package` → ES modules; `Vector.<T>` → `Array<T>`; `Dictionary` → `Map`; events → EventEmitter or DOM events.
7. For Adobe AIR apps (still supported): AIR allows AS3 desktop/mobile apps — use Harman's AIR SDK for continued support; consider migrating to Electron (desktop) or React Native (mobile) for long-term viability.
8. Apache Royale (formerly FlexJS): compiles AS3/MXML to JavaScript — provides a migration path for Flex applications to HTML5 without complete rewrites; useful as an intermediate step.
