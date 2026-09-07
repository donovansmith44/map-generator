Feature: the contract endpoint — a server declares what it speaks
  A consumer refuses version skew instead of discovering it. The whole
  body is pinned; the graph pin varies with the compiled canon by
  design, so it is masked HERE, visibly, and shape-checked instead.

  Vocabulary:
    | mask-shape | any of: sixteen hex characters |

  Scenario: the contract declaration is exactly its blessed body
    When I GET /api/contract
    Then the response equals fixture "contract" masking graphPin as sixteen hex characters
