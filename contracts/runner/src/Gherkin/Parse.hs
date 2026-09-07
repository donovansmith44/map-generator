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
          goBody rest (Feature t tags [] [] [])
      | otherwise = err n ("expected Feature:, got " <> strip l)

    tagsOf l = [ Tag (T.drop 1 w) | w <- T.words (strip l), "@" `T.isPrefixOf` w ]

    goBody [] f = Right (doneFeature f)
    goBody ls@((_, l) : rest) f
      | isComment l = goBody rest f
      | strip l == "Vocabulary:" =
          case vocabRows rest of
            Left e -> Left e
            Right (vs, rest') -> goBody rest' f { ftVocab = ftVocab f ++ vs }
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

    goScenarios [] f acc = Right (doneFeature f { ftScenarios = ftScenarios f ++ reverse acc })
    goScenarios ((n, l) : rest) f acc
      | isComment l = goScenarios rest f acc
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

    doneFeature = id
