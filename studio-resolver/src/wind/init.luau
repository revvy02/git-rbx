--[[
	Vendored miu-wind (revvy02/miu-wind 0.1.0) style primitives + WindRules,
	with string requires so rodeo's darklua bundling can resolve it — the
	pesde shims use instance-path requires that don't survive single-file
	bundling. px is inlined from revvy02/miu-px 0.1.0 for the same reason.
	ThemeRules/MarkdownRules are not vendored (unused here).
]]
return {
	StyleRule = require("@self/StyleRule"),
	StyleLink = require("@self/StyleLink"),
	StyleSheet = require("@self/StyleSheet"),
	WindRules = require("@self/WindRules"),
}
