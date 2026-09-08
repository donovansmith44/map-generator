module Gherkin.Parse (parseFeature) where

import Control.Applicative ((<|>))
import Data.Text (Text)
import qualified Data.Text as T
import Gherkin.Ast

-- Line-oriented: classify each stripped line, fold into the AST.
-- And/But resolve to the PREVIOUS keyword here, so the AST never
-- carries them — a law the round-trip property preserves (render
-- never emits And; parse of an And-free file is identity).
parseFeature :: FilePath -> Text -> Either Text Feature
parseFeature path src = go0 (zip [1 :: Int ..] (map T.stripEnd (T.lines src))) []
  where
    err n m = Left (T.pack path <> ":" <> T.pack (show n) <> " " <> m)
    strip = T.strip
    isComment l = "#" `T.isPrefixOf` strip l || T.null (strip l)

    go0 [] _ = Left (T.pack path <> ": no Feature line")
    go0 ((n, l) : rest) tags
      | isComment l = go0 rest tags
      | "@" `T.isPrefixOf` strip l = go0 rest (tags ++ tagsOf l)
      | Just t <- T.stripPrefix "Feature: " (strip l) =
          goBody rest (Feature t tags [] [] [] [])
      | otherwise = err n ("expected Feature:, got " <> strip l)

    tagsOf l = [ Tag (T.drop 1 w) | w <- T.words (strip l), "@" `T.isPrefixOf` w ]

    goBody [] f = Right (doneFeature f)
    goBody ls@((n, l) : rest) f
      | isComment l = goBody rest f
      | strip l == "Vocabulary:" =
          case vocabRows rest of
            Left e -> Left e
            Right (vs, rest') -> goBody rest' f { ftVocab = ftVocab f ++ vs }
      -- R97: the background sits after the description (and its
      -- Vocabulary table) and before the first scenario. Reached only
      -- from `goBody`, which is left for good the moment a tag or
      -- Scenario line appears -- so a Background AFTER a scenario can
      -- never be parsed as one, and is caught by `goScenarios`'s own
      -- "expected Scenario or tags" arm with the line named.
      | strip l == "Background:" =
          if not (null (ftBackground f))
            -- Requirement 4: a second background is an ERROR, not a
            -- silent last-wins. Last-wins is the shape where a file
            -- says one thing and the runner does another, which is
            -- precisely what a corpus may not do.
            then err n "a feature may have only one Background:"
            else case goBackground rest [] Nothing of
              Left e -> Left e
              Right (sts, rest') -> goBody rest' f { ftBackground = sts }
      | "@" `T.isPrefixOf` strip l || "Scenario: " `T.isPrefixOf` strip l =
          goScenarios ls f []
      | otherwise = goBody rest f { ftPreamble = ftPreamble f ++ [strip l] }

    vocabRows :: [(Int, Text)] -> Either Text ([(Text, Text)], [(Int, Text)])
    vocabRows ((n, l) : rest)
      | Just row <- tableRow l =
          case row of
            [k, v] -> do
              (vs, rest') <- vocabRows rest
              pure ((k, v) : vs, rest')
            _ -> err n ("malformed vocabulary row (expected 2 columns, got "
                        <> T.pack (show (length row)) <> "): " <> strip l)
    vocabRows ls = Right ([], ls)

    tableRow l =
      let s = strip l
      in if "|" `T.isPrefixOf` s && "|" `T.isSuffixOf` s && T.length s > 1
           then Just (map T.strip (T.splitOn "|" (T.dropEnd 1 (T.drop 1 s))))
           else Nothing

    -- The background's own steps, read with exactly the step grammar a
    -- scenario's body uses -- And/But resolve against the previous
    -- keyword here too, so a background is not a second, subtly
    -- different dialect. Ends at the first tag or Scenario line, which
    -- is handed back unconsumed.
    goBackground :: [(Int, Text)] -> [Step] -> Maybe Keyword
                 -> Either Text ([Step], [(Int, Text)])
    goBackground [] acc _ = Right (acc, [])
    goBackground ls@((n, l) : rest) acc prevKw
      | isComment l = goBackground rest acc prevKw
      | "@" `T.isPrefixOf` strip l || "Scenario: " `T.isPrefixOf` strip l =
          Right (acc, ls)
      | strip l == "Background:" = err n "a feature may have only one Background:"
      -- a table row attaches to the step just read, exactly as it does
      -- inside a scenario body
      | Just row <- tableRow l
      , (s0 : older) <- reverse acc =
          goBackground rest
            (reverse (s0 { stepArg = Just (addRowB (stepArg s0) row) } : older)) prevKw
      | otherwise =
          case kwOf (strip l) prevKw of
            Just (k, b) -> goBackground rest (acc ++ [Step k b Nothing]) (Just k)
            Nothing -> err n ("not a step: " <> strip l)
      where
        addRowB (Just (Table rs)) r = Table (rs ++ [r])
        addRowB _ r = Table [r]

    goScenarios [] f acc = Right (doneFeature f { ftScenarios = ftScenarios f ++ reverse acc })
    goScenarios ((n, l) : rest) f acc
      | isComment l = goScenarios rest f acc
      -- A Background reached from here is one that comes AFTER a
      -- scenario. It is an error either way (nothing below accepts the
      -- line), but a reader deserves the reason rather than "not a
      -- step": a background sets up the scenarios that follow it, so a
      -- background with nothing following it is a contradiction.
      | strip l == "Background:" = err n backgroundTooLate
      | "@" `T.isPrefixOf` strip l =
          goScenario rest f acc (tagsOf l) n
      | Just t <- T.stripPrefix "Scenario: " (strip l) =
          goSteps rest f acc (Scenario t [] []) Nothing
      | otherwise = err n ("expected Scenario or tags, got " <> strip l)
      where
        goScenario ((n', l') : rest') f' acc' tg _
          | isComment l' = goScenario rest' f' acc' tg n'
          | Just t <- T.stripPrefix "Scenario: " (strip l') =
              goSteps rest' f' acc' (Scenario t tg []) Nothing
        goScenario _ _ _ _ n' = err n' "tags must precede a Scenario"

    goSteps [] f acc sc _ =
      Right (doneFeature f { ftScenarios = ftScenarios f ++ reverse (sc : acc) })
    goSteps ls@((n, l) : rest) f acc sc prevKw
      | isComment l = goSteps rest f acc sc prevKw
      | strip l == "Background:" = err n backgroundTooLate
      | "@" `T.isPrefixOf` strip l || "Scenario: " `T.isPrefixOf` strip l =
          goScenarios ls f (sc : acc)
      | Just row <- tableRow l
      , (s0 : older) <- reverse (scSteps sc) =
          let s0' = s0 { stepArg = Just (addRow (stepArg s0) row) }
          in goSteps rest f acc sc { scSteps = reverse (s0' : older) } prevKw
      | otherwise =
          case kwOf (strip l) prevKw of
            Just (k, b) ->
              goSteps rest f acc sc { scSteps = scSteps sc ++ [Step k b Nothing] } (Just k)
            Nothing -> err n ("not a step: " <> strip l)
      where
        addRow (Just (Table rs)) r = Table (rs ++ [r])
        addRow _ r = Table [r]

    kwOf s prev =
          (,) Given <$> T.stripPrefix "Given " s
      <|> (,) When  <$> T.stripPrefix "When "  s
      <|> (,) Then  <$> T.stripPrefix "Then "  s
      <|> (do b <- T.stripPrefix "And " s <|> T.stripPrefix "But " s
              k <- prev
              pure (k, b))

    backgroundTooLate =
      "Background: must come before the first Scenario: it sets up the "
      <> "scenarios that follow it"

    doneFeature = id
