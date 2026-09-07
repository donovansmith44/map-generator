module Main where

import Test.Hspec
import Gherkin.Ast

main :: IO ()
main = hspec $
  describe "scaffold" $
    it "keywords enumerate Given/When/Then" $
      [minBound .. maxBound] `shouldBe` [Given, When, Then]
