Feature: the camera — what you look at is what you get, and nothing else changes
  Changing the view means one thing: choosing which features are sent.
  It never edits them. VISIBILITY IS DEFINED, not vibes: the view is a
  cap on the globe at the given center whose radius the zoom declares
  (radius = zoom x 1.8 degrees, the server's own margin, confirmed
  empirically at 1.78 in / 1.82 out); a feature is IN VIEW when its own
  bounding cap intersects the view cap, OUT OF VIEW when the two caps
  are disjoint beyond the margin, and BEYOND THE HORIZON when its
  bounds lie entirely more than a quarter turn from the center. The
  step definitions compute these predicates from the manifest's own
  bounds; no test names a landmark by hand. Detail is pinned explicitly
  wherever the camera is under test, because zoom otherwise changes
  detail too (detail.feature owns that coupling law).

  Vocabulary:
    | pieces | any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water |
    | year | whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist) |
    | style | any of: canaan, parchment, slate |
    | detail | any of: coarse, fine, ultra |
    | scale | any of: doubled, halved |

  Scenario: looking at the Levant shows the Levant
    When I render pieces all at year -1405 in style canaan looking at 31.5,35.0 zoom 4 detail fine
    Then the response equals fixture "scene-1405-levant-cam"

  # Characterization: regions and boundaries are never culled today —
  # 917 feature ids at every zoom, even for an antipodal camera. This
  # law states what a camera SHOULD mean; it is red until culling is
  # real, and no weaker law is worth pinning.
  @target @property
  Scenario: everything in view is sent, and nothing far beyond the view is
    When I render pieces <somePieces> at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> detail fine as viewed
    And I render pieces <somePieces> at year <someYear> in style <someStyle> detail fine as world
    Then viewed keeps every feature of world in view and omits every feature of world out of view

  @property
  Scenario: moving the camera never redraws what stays visible
    When I render pieces <somePieces> at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> detail fine as here
    And I render pieces <somePieces> at year <someYear> in style <someStyle> looking at <someOtherCenter> zoom <someZoom> detail fine as there
    Then every resource here and there share is byte-identical in both

  # Green today for the kinds the camera actually culls (markers 9->31
  # and labels 833->856 across the zoom ladder, zero violations). The
  # feature-level version of this law is the @target above.
  @property
  Scenario: zooming out only reveals markers and labels — it never removes them
    When I render pieces <somePieces> at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> detail fine as narrow
    And I render pieces <somePieces> at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> doubled detail fine as wide
    Then narrow's markers and labels are a subset of wide's

  @property
  Scenario: zooming in never loses a marker or label you are looking at
    When I render pieces <somePieces> at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> detail fine as wide
    And I render pieces <somePieces> at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> halved detail fine as narrow
    Then every marker and label of wide still in narrow's view is kept by narrow

  # Characterization: an antipodal camera still receives all 917 ids.
  @target @property
  Scenario: the far side of the globe is never sent
    When I render pieces <somePieces> at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> detail fine as viewed
    Then no feature of viewed is beyond the horizon of <someCenter>

  # The screenshots' law. Characterization: 99.3% of labels (827/833)
  # anchor outside a 0.09-degree viewport, the farthest 148 degrees
  # away — the GPU path does no label-anchor culling at all.
  @target @property
  Scenario: a label is only sent when the thing it names is in view
    When I render pieces <somePieces> at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> detail fine as viewed
    Then every label of viewed anchors in view

  @property
  Scenario: the same view twice is the same view
    When I render pieces <somePieces> at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as first
    And I render pieces <somePieces> at year <someYear> in style <someStyle> looking at <someCenter> zoom <someZoom> as second
    Then first equals second
