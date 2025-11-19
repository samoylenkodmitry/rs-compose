# RS-Compose Development Roadmap

**Last Updated**: November 2025
**Modifier System Status**: ✅ Jetpack Compose Parity Achieved
**Documentation Status**: ✅ Comprehensive Internals Documented

---

## ✅ Completed Milestones

### Modifier System (Nov 2025)
- ✅ `ModifierNodeEntry` stores `Rc<RefCell<Box<dyn ModifierNode>>>`
- ✅ `LayoutModifierNode::measure` returns `LayoutModifierMeasureResult` with placement control
- ✅ `LayoutModifierCoordinator` holds live node references (no proxy bypass)
- ✅ Chain preservation - no flattening that loses modifier interleaving
- ✅ Node lifecycle: `on_attach()`, `on_detach()`, `on_reset()`
- ✅ Capability-based dispatch with bitflags
- ✅ All core modifiers implemented: Padding, Size, Offset, Background, Clickable, etc.

### Documentation (Nov 2025)
- ✅ **MODIFIERS.md** (37KB) - Complete modifier system internals
- ✅ **SNAPSHOTS_AND_SLOTS.md** (54KB) - Snapshot and slot table internals
- ✅ **modifier_match_with_jc.md** - Parity status tracking
- ✅ GitHub Actions workflow for CI

### Test Coverage
- ✅ 460+ workspace tests passing
- ✅ Modifier chain composition and reuse
- ✅ Layout measurement and placement protocols
- ✅ Capability-based dispatch validation
- ✅ Integration with draw, pointer input, semantics

---

## 🎯 Current Priorities (Q4 2025)

### 1. Real-World Application Development
**Goal**: Build production-quality example apps to validate the framework

- [ ] **Complex Desktop Application** - Multi-window IDE/editor-style app
  - State management across windows
  - Complex nested layouts with performance testing
  - Drag-and-drop, keyboard shortcuts
  - Custom rendering for code editor

- [ ] **Dashboard/Data Visualization App**
  - Charts, graphs, real-time data updates
  - LazyColumn/LazyRow with large datasets
  - Scrolling performance optimization

- [ ] **Form-Heavy Application**
  - Text input validation
  - Focus management
  - Tab navigation
  - Error states and accessibility

**Why**: Real applications expose edge cases and inform API improvements

### 2. Performance Optimization & Benchmarking
**Goal**: Establish performance baselines and optimize hot paths

- [ ] **Benchmark Suite** (Priority: High)
  - Modifier chain traversal (various depths)
  - Recomposition with different invalidation patterns
  - Layout measurement with complex trees
  - Draw command generation
  - Memory allocation profiling

- [ ] **Optimization Targets**
  - Cache aggregated node capabilities (avoid recomputation)
  - Pool allocations in `update_from_slice`
  - Optimize `SnapshotIdSet` operations for common patterns
  - Reduce `RefCell` borrow overhead where safe

- [ ] **Profiling Infrastructure**
  - Flamegraph integration for layout/draw passes
  - Frame time tracking
  - Memory usage tracking

**Why**: Performance is critical for user experience; baselines prevent regressions

### 3. Missing Core Features
**Goal**: Fill gaps in functionality to match Jetpack Compose completeness

- [ ] **Intrinsic Measurements** (High Priority)
  - Implement `IntrinsicMeasureScope` for Row/Column
  - `IntrinsicSize.Min`, `IntrinsicSize.Max` modifiers
  - Baseline alignment support

- [ ] **Text Editing** (High Priority)
  - TextField/OutlinedTextField composables
  - Cursor positioning and selection
  - IME integration (desktop, mobile)
  - Text selection gestures

- [ ] **Lazy Layouts Performance**
  - Viewport recycling optimization
  - Item key stability and reuse
  - Prefetching and smooth scrolling

- [ ] **Animation System**
  - `animate*AsState` helpers
  - `Animatable` and `Transition` APIs
  - Animated modifiers (size, offset, color, etc.)
  - Spring physics

**Why**: These features are essential for building real applications

### 4. Developer Experience Improvements
**Goal**: Make the framework easier to learn and use

- [ ] **Enhanced Error Messages**
  - Better `RefCell` borrow panic context (which node, which operation)
  - Layout measurement constraint violations with debugging hints
  - Composition cycle detection with stack traces

- [ ] **Developer Tools**
  - Layout inspector (visualize layout tree)
  - Recomposition counter/highlighter
  - Performance overlay (frame times, skip counts)

- [ ] **Examples & Guides**
  - Getting started tutorial (counter → todo list → custom modifier)
  - Migration guide from immediate-mode UIs
  - Performance best practices guide
  - Custom layout guide

**Why**: Good DX accelerates adoption and reduces frustration

### 5. Platform Support Expansion
**Goal**: Enable cross-platform development

- [ ] **Android Support** (Medium Priority)
  - `ndk_glue` integration
  - Skia renderer on Android surface
  - Touch input handling
  - IME support

- [ ] **Web Support** (Lower Priority)
  - WebAssembly compilation
  - Canvas/WebGL backend
  - Web event integration

- [ ] **iOS Support** (Future)
  - UIKit integration
  - Metal rendering
  - Touch and gesture handling

**Why**: Cross-platform is a key Rust value proposition

---

## 📊 Success Metrics

- **Performance**: 60 FPS on complex desktop apps (10k+ layout nodes)
- **Developer Satisfaction**: Positive feedback from early adopters
- **Test Coverage**: >80% line coverage on core crates
- **Documentation**: Every public API has doc comments with examples
- **Ecosystem**: 3+ community-contributed crates (themes, components, etc.)

---

## 🔄 Review Schedule

This roadmap is reviewed and updated:
- **Monthly**: Adjust priorities based on learnings
- **After Major Milestones**: Celebrate wins, identify blockers
- **Based on Feedback**: Community input shapes direction

---

## 📚 Reference Documentation

- [ARCHITECTURE.md](docs/ARCHITECTURE.md) - Overall framework design
- [MODIFIERS.md](MODIFIERS.md) - Modifier system internals
- [SNAPSHOTS_AND_SLOTS.md](SNAPSHOTS_AND_SLOTS.md) - Runtime internals
- [modifier_match_with_jc.md](modifier_match_with_jc.md) - Jetpack Compose parity status

---

## 💡 Immediate Next Actions

**Recommended starting point**: Performance benchmarking
1. Set up Criterion benchmarks for modifier chain operations
2. Profile the desktop-demo app to identify bottlenecks
3. Establish baseline metrics before optimization
4. Document findings in `docs/PERFORMANCE.md`

**Alternative**: Build a real application
1. Choose a target app (e.g., markdown editor, kanban board)
2. Implement core features using existing APIs
3. Document pain points and missing features
4. Use findings to prioritize API improvements
