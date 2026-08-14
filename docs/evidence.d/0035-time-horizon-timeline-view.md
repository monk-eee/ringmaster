# EV-0035: Time Horizon timeline view — an alternative, zoomable presentation of the existing bucketed data

Evidence for [ADR-0035](../adr.d/0035-time-horizon-timeline-view.md).

State is derived by `node scripts/check-evidence.mjs` and is deliberately not
written here.

```toml
adr = "0035-time-horizon-timeline-view"

[[check]]
id = "timeline-view-toggle-exists"
invariant = "A Buckets/Timeline view toggle exists on the Time Horizon tab."
type = "present"
pattern = 'time-horizon-view-toggle'
paths = ["frontend/src/components/TimeHorizon.tsx"]

[[check]]
id = "timeline-renders-bands-with-existing-accents"
invariant = "The timeline renders bands using the existing accent classes rather than a new color scheme."
type = "present"
pattern = 'accent-\$\{band\.accent\}'
paths = ["frontend/src/components/TimeHorizonTimeline.tsx"]

[[check]]
id = "timeline-stacks-same-day-items-with-count"
invariant = "Obligations sharing the same effective due date collapse into one marker with a count."
type = "present"
pattern = 'groupByEffectiveDate'
paths = ["frontend/src/components/TimeHorizonTimeline.tsx"]

[[check]]
id = "timeline-marker-reveals-evidence-reason-on-click"
invariant = "Clicking a marker reveals its evidence-backed reason inline, reusing the existing row presentation."
type = "present"
pattern = 'expandedStack'
paths = ["frontend/src/components/TimeHorizonTimeline.tsx"]

[[check]]
id = "timeline-supports-pan-focus-and-zoom-and-now-reset"
invariant = "Pan-by-focus, two-state zoom, and a Now reset are all implemented."
type = "present"
pattern = 'handleNow'
paths = ["frontend/src/components/TimeHorizonTimeline.tsx"]

[[check]]
id = "playwright-proves-timeline-interaction"
invariant = "Focused browser coverage proves switching to Timeline, expanding a stack, and using Now/zoom/pan."
type = "present"
pattern = 'time horizon: switching to Timeline view'
paths = ["frontend/tests/obligations.spec.ts"]
```

## Notes

All six checks are automated against the implementing component/test. No
backend route, migration, or dependency change is part of this ADR, so no
`cargo test` evidence applies here.
