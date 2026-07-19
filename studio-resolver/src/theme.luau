--[[
	VS Code-dark theme, lifted from the old rbx-diff-viewer frontend
	(frontend/src/styles/index.css): #1e1e1e/#252526 surfaces, 1px #404040
	borders, monospace throughout, small radii, #094771 selection, #0e639c
	action blue, and the VS Code semantic palette for diff states.
]]

local StyleRule = require("@self/wind/StyleRule")

local theme = {}

-- Surfaces
theme.bg = Color3.fromRGB(30, 30, 30) -- #1e1e1e
theme.surface = Color3.fromRGB(37, 37, 38) -- #252526
theme.surfaceHigh = Color3.fromRGB(45, 45, 45) -- #2d2d2d
theme.border = Color3.fromRGB(64, 64, 64) -- #404040
theme.hover = Color3.fromRGB(42, 45, 46) -- #2a2d2e
theme.selection = Color3.fromRGB(9, 71, 113) -- #094771

-- Text
theme.text = Color3.fromRGB(212, 212, 212) -- #d4d4d4
theme.textBright = Color3.fromRGB(255, 255, 255)
theme.textMuted = Color3.fromRGB(128, 128, 128) -- #808080
theme.textDim = Color3.fromRGB(96, 96, 96) -- #606060

-- Actions
theme.accent = Color3.fromRGB(14, 99, 156) -- #0e639c
theme.accentHover = Color3.fromRGB(17, 119, 187) -- #1177bb
theme.neutralButton = Color3.fromRGB(60, 60, 60) -- #3c3c3c
theme.disabled = Color3.fromRGB(64, 64, 64) -- #404040

-- Diff semantics (VS Code): sides + states
theme.added = Color3.fromRGB(78, 201, 176) -- #4ec9b0
theme.removed = Color3.fromRGB(241, 76, 76) -- #f14c4c
theme.modified = Color3.fromRGB(220, 220, 170) -- #dcdcaa
theme.propName = Color3.fromRGB(156, 220, 254) -- #9cdcfe

theme.unresolved = theme.modified
theme.ours = theme.propName -- #9cdcfe
theme.theirs = Color3.fromRGB(197, 134, 192) -- #c586c0 (VS Code magenta)
theme.custom = theme.added

-- RichText hexes for the side colors
theme.oursHex = "#9CDCFE"
theme.theirsHex = "#C586C0"

-- Typography: monospace everywhere, like the viewer's Consolas
theme.font = Enum.Font.RobotoMono
theme.fontBold = Enum.Font.RobotoMono
theme.textSize = 13
theme.textSizeSmall = 11
theme.headerSize = 11

-- Resolver style classes layered on top of WindRules.
function theme.rules()
	StyleRule(".surface", { BackgroundColor3 = theme.surface, BackgroundTransparency = 0 })
	StyleRule(".surface-high", { BackgroundColor3 = theme.surfaceHigh, BackgroundTransparency = 0 })
	StyleRule(".vs-border::UIStroke", {
		Thickness = 1,
		Transparency = 0,
		Color = theme.border,
		BorderStrokePosition = Enum.BorderStrokePosition.Inner,
	})
end

return theme
