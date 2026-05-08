# Comments policy

doclick was built feature-first by AI agents and was carrying hundreds of low-value comments by the time of the production-readiness pass. The policy below codifies what survived and what we want to keep out.

## Default: no comment

Well-named identifiers and small, self-evident functions don't need comments. **Removing a comment is the default move.** Only write a comment when removing it would actively make the code harder to understand for the next reader.

## Write a comment only when it documents one of these

1. **A non-obvious architectural decision.**
   _Example (state.rs):_ `Intentionally not Clone — clones must go through AppState::clone() so every owner sees the same locked state.`

2. **A complex algorithm or formula.**
   _Example (translate.rs):_ the proportional-mapping math has a one-line "what" comment because the variable names alone don't communicate the intent.

3. **A subtle invariant or pitfall a future reader would re-introduce.**
   _Example (App.tsx):_ `Apply size BEFORE the view flip so the settings UI never paints at overlay dimensions.`

4. **A public API doc — `///` rustdoc on Rust items, `/** */` JSDoc on TS exports.**
   These are the API surface. Document parameters, return values, locking implications.

5. **A Win32 quirk or platform-specific gotcha.**
   _Example (lib.rs):_ `-32000 is the Win32 sentinel for a minimized window's GetWindowPos result; skip it to avoid spawning offscreen.`

6. **An eslint/clippy rule disable, with a 1-line justification.**
   _Example (eslint.config.js):_ the `set-state-in-effect: "off"` block has a short rationale.

7. **A timing constant whose value isn't obvious from context.**
   _Example (broadcast/mod.rs):_ each `Duration` field on `BroadcastTimings` carries a doc comment explaining what would break if it were lower.

## Delete on sight

These are slop. Strip them aggressively when you find them.

### Restating obvious code

```rust
// BAD: the next line says exactly this.
i += 1; // increment counter

// BAD:
inner.profiles.push(profile.clone());
// Append to ordering if not already there.
if !inner.profile_order.contains(&profile.id) {
```

### Narrating the implementation flow

```rust
// BAD: agent narrating its own work.
// First, validate the input.
// Then, we lock the state.
// Now we update the field.
// Finally, persist.
```

### Referencing the current PR / fix / task

```rust
// BAD: rots immediately after merge.
// Added in PR #23 to fix focus loss.
// TODO from initial impl
// fixes issue with focus
```

### Stale `// TODO` / `// FIXME` with no concrete owner

If the TODO has been there for more than a sprint and nobody's claimed it, delete the TODO and either fix the code or document the constraint as an architectural note.

### `// removed X` placeholders

```rust
// BAD: git log already knows.
// removed: legacy main role coercion
// migrated from MutexGuard to RwLockReadGuard
```

When you remove code, remove it. Don't leave a tombstone.

### Decorative banners and section markers

```rust
// BAD: editor folding does this for you.
// =================== Layout constants ===================
// SECTION: actions
// --- helpers ---
```

(One subtle exception: a one-line comment introducing a logically grouped block — `// Mouse-bound shortcuts only — never left/right buttons.` — is fine when it documents *why* the block is grouped that way.)

### Repeating what a well-named identifier says

```ts
// BAD:
const appHandle: AppHandle = ...;  // the app handle
const visibleCount = ...;          // count of visible chars
```

## Comment style cheat sheet

- **Rust public items:** `///` rustdoc.
- **Rust private items:** prefer no comment; if you need one, plain `//`.
- **TS exports / public types:** `/** ... */` JSDoc.
- **TS internal:** plain `//`.
- **Multi-line architectural notes:** wrap at ~78 columns; lead with the *why*, end with the *consequence* of getting it wrong.
- **No emojis.** No "Happy coding!".

## When in doubt

Ask: *"If I deleted this comment, would the next reader misunderstand the code?"*

- If yes — keep it, and tighten the wording to the minimum useful form.
- If no — delete it.

If the answer is "they'd misunderstand because the code is too clever", the better move is usually to refactor the code. Comments are not a substitute for clear code.
