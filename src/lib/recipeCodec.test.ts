import { describe, expect, it } from "vitest";
import { supportedFilmSimulations } from "../domain/recipe";
import { normalizeRecipe, parseRecipeCollectionText, parseRecipeText } from "./recipeCodec";

describe("Recipe import normalization", () => {
  it("retains the complete verified X-M5 film-simulation set", () => {
    expect(supportedFilmSimulations).toHaveLength(20);
    for (const filmSimulation of supportedFilmSimulations) {
      expect(normalizeRecipe({ settings: { filmSimulation } }).settings.filmSimulation).toBe(
        filmSimulation,
      );
    }
  });

  it("parses bilingual Recipe text and preserves half-stop tone values", () => {
    const recipe = parseRecipeText(`
名稱：測試 Recipe
軟片模擬：Classic Chrome
白平衡：日光，R +1／B -2
高光：-0.5
陰影：+1.5
顆粒：Strong, Large
`);
    expect(recipe.name).toBe("測試 Recipe");
    expect(recipe.settings.filmSimulation).toBe("CLASSIC_CHROME");
    expect(recipe.settings.whiteBalance).toMatchObject({
      mode: "DAYLIGHT",
      shiftR: 1,
      shiftB: -2,
    });
    expect(recipe.settings.highlight).toBe(-0.5);
    expect(recipe.settings.shadow).toBe(1.5);
    expect(recipe.settings.grain).toEqual({ strength: "STRONG", size: "LARGE" });
  });

  it("accepts collections only when explicitly supplied as JSON", () => {
    const recipes = parseRecipeCollectionText(
      JSON.stringify([
        { id: "one", name: "One", settings: { filmSimulation: "ASTIA" } },
        { id: "two", name: "Two", settings: { filmSimulation: "REALA_ACE" } },
      ]),
    );
    expect(recipes.map((recipe) => recipe.name)).toEqual(["One", "Two"]);
    expect(recipes.map((recipe) => recipe.settings.filmSimulation)).toEqual([
      "ASTIA",
      "REALA_ACE",
    ]);
  });
});
