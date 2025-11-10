# Planning Session - November 2025

This document outlines planned improvements and new features for the htmxology framework based on recent developments and user needs.

## Recent Achievements

- ✅ Clone trait support for HtmlId and HtmlName (v0.20.0)
- ✅ Pre-handler support in RoutingController macro
- ✅ Design document for extra_derives feature (Issue #24)
- ✅ Better Args semantic for parameterized controllers
- ✅ Removal of restrictive From implementation

## Planned Features

### 1. RoutingController Extra Derives (Issue #24)

**Priority**: High
**Status**: Design Complete, Ready for Implementation

Implement the `extra_derives` parameter for the `RoutingController` macro to allow users to add custom derive traits to generated Route enums.

**Benefits**:
- Enable routes as HashMap/HashSet keys with `Hash`, `Eq`
- Support route serialization with `serde::Serialize`/`Deserialize`
- Allow route comparisons and ordering
- Fully backward compatible

**Implementation Steps**:
1. Update `ControllerSpec` structure to store extra derives
2. Extend parser to recognize `extra_derives(...)` syntax
3. Modify route enum generation to include extra derives
4. Add custom `extra_derives` keyword
5. Add comprehensive snapshot and integration tests
6. Update documentation (CLAUDE.md, API docs)

**Timeline**: 1-2 weeks

### 2. Enhanced Error Handling

**Priority**: Medium
**Status**: Concept

Improve error handling and debugging experience:

**Proposed Improvements**:
- Better error messages in derive macros with span information
- Helper traits for converting between controller response types
- Optional error tracking/logging middleware
- Typed error responses with HTMX-aware error rendering

**Use Cases**:
- Development debugging
- Production error monitoring
- User-friendly error pages with partial updates
- API error responses with proper status codes

### 3. Middleware System

**Priority**: Medium
**Status**: Research

Design a middleware system for controllers:

**Capabilities**:
- Pre-request processing (authentication, logging)
- Post-response processing (compression, caching)
- Error transformation
- Request/response interception
- Composition with Axum middleware

**Example API**:
```rust
#[derive(RoutingController)]
#[controller(AppRoute)]
#[middleware(AuthMiddleware, LoggingMiddleware)]
#[subcontroller(BlogController, route = Blog, path = "blog/")]
struct AppController {
    state: AppState,
}
```

### 4. WebSocket Integration Improvements

**Priority**: Low
**Status**: Concept

Enhance WebSocket support with HTMX integration:

**Features**:
- Type-safe WebSocket routes
- HTMX SSE (Server-Sent Events) helpers
- Real-time component updates
- Connection state management

**Use Cases**:
- Live notifications
- Real-time dashboards
- Collaborative editing
- Chat applications

### 5. Form Validation Helpers

**Priority**: Medium
**Status**: Concept

Provide utilities for server-side form validation with HTMX:

**Features**:
- Validation trait for form types
- Automatic error rendering with HTMX attributes
- Partial form validation (validate on blur)
- Integration with validator crates
- Type-safe form field targeting

**Example**:
```rust
#[derive(Validate, FormData)]
struct LoginForm {
    #[validate(email)]
    email: String,

    #[validate(length(min = 8))]
    password: String,
}

// Automatically generates HTMX validation routes
// POST /validate/email -> validates email field only
// POST /submit -> validates entire form
```

### 6. Development Tools

**Priority**: Low
**Status**: Concept

Enhance developer experience:

**Tools**:
- Route visualization/debugging panel
- Request/response inspector
- Auto-reload improvements
- Template hot-reloading
- Performance profiling helpers

### 7. Testing Utilities

**Priority**: Medium
**Status**: Concept

Provide testing helpers for htmxology applications:

**Features**:
- Mock HTMX request builders
- Controller test harness
- Snapshot testing for rendered HTML
- Integration test helpers
- Route coverage analysis

**Example**:
```rust
#[tokio::test]
async fn test_blog_route() {
    let controller = BlogController::new(mock_state());
    let request = HtmxRequest::builder()
        .target("#blog-content")
        .boosted()
        .build();

    let response = controller
        .handle(BlogRoute::List, request)
        .await
        .unwrap();

    assert_html_snapshot!(response);
}
```

### 8. Documentation Improvements

**Priority**: High
**Status**: Ongoing

**Tasks**:
- Comprehensive examples for all features
- Tutorial series (beginner to advanced)
- Architecture guide for large applications
- Performance best practices
- Migration guides between versions
- API reference completeness

### 9. Performance Optimizations

**Priority**: Medium
**Status**: Research

**Areas**:
- Route matching optimization
- Template caching strategies
- Response streaming
- Lazy controller initialization
- Compile-time route validation

### 10. Community Features

**Priority**: Low
**Status**: Concept

**Ideas**:
- Plugin system for reusable components
- Template library (common UI patterns)
- Authentication providers
- Database integration examples
- Deployment guides

## Technical Debt

### Code Quality
- Improve macro error messages
- Reduce code duplication in examples
- Add more inline documentation
- Refactor complex functions

### Testing
- Increase test coverage for edge cases
- Add property-based tests for route parsing
- Performance benchmarks
- Fuzzing for macro inputs

### Dependencies
- Regular dependency updates
- Security audit automation
- License compliance checks (already in place)

## Release Planning

### v0.21.0 (Target: Q4 2025)
- RoutingController extra_derives feature
- Enhanced error handling
- Documentation improvements
- Bug fixes

### v0.22.0 (Target: Q1 2026)
- Middleware system
- Form validation helpers
- Testing utilities
- Performance optimizations

### v1.0.0 (Target: Q2 2026)
- API stabilization
- Complete documentation
- Production-ready status
- Long-term support commitment

## Community Engagement

**Outreach**:
- Blog posts about HTMX + Rust SSR
- Conference talks
- Tutorial videos
- Example applications showcase

**Feedback Channels**:
- GitHub issues
- Discussions
- User surveys
- Community Discord/Slack

## Success Metrics

**Adoption**:
- Downloads per month
- GitHub stars
- Community contributions
- Projects using htmxology

**Quality**:
- Test coverage > 85%
- Documentation coverage 100%
- Zero critical bugs
- Fast issue response time

**Performance**:
- Route matching < 1μs
- Template rendering benchmarks
- Memory usage profiling
- Minimal dependency footprint

## Next Steps

1. **Immediate** (This Week):
   - Implement extra_derives feature
   - Add more examples
   - Review and triage open issues

2. **Short-term** (This Month):
   - Complete extra_derives testing
   - Design middleware system
   - Plan form validation API

3. **Medium-term** (This Quarter):
   - Release v0.21.0
   - Start middleware implementation
   - Expand documentation

4. **Long-term** (6+ Months):
   - Stabilize API for v1.0.0
   - Build plugin ecosystem
   - Grow community

## Notes

This planning document is a living document and should be updated as priorities change, new features are identified, or community feedback is received.

Last updated: 2025-11-10
