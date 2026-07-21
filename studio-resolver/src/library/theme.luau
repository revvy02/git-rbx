--[[
	VS Code-dark theme, lifted from the old rbx-diff-viewer frontend
	(frontend/src/styles/index.css): #1e1e1e/#252526 surfaces, 1px #404040
	borders, monospace throughout, small radii, #094771 selection, #0e639c
	action blue, and the VS Code semantic palette for diff states.
]]

local wind = require("../packages").wind
local StyleRule = wind.StyleRule

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

-- RichText hex for dim metadata.
theme.textDimHex = "#606060"

function theme.toHex(color: Color3): string
	return string.format("#%02X%02X%02X", color.R * 255, color.G * 255, color.B * 255)
end

-- Staged-resolution status color (nil = unpicked/unresolved).
function theme.statusColor(resolution: string?): Color3
	if resolution == "ours" then
		return theme.ours
	elseif resolution == "theirs" then
		return theme.theirs
	elseif resolution == "custom" then
		return theme.custom
	end
	return theme.unresolved
end

-- Typography: monospace everywhere, like the viewer's Consolas
theme.font = Enum.Font.RobotoMono
theme.fontBold = Enum.Font.RobotoMono
theme.textSize = 14
theme.textSizeSmall = 12
theme.headerSize = 12

-- Each render surface needs its own sheet link: the wind utility classes
-- (flex-*, gap-*, p-*, corner-*, auto-size-*, bg-none, ...) plus the
-- resolver's VS Code-dark classes below.
function theme.link()
	debug.profilebegin("rbxdiff_sheet")
	wind.StyleLink(wind.StyleSheet(function()
		wind.WindRules()
		theme.rules()
	end))
	debug.profileend()
end

-- Resolver style classes layered on top of WindRules, after miu-wind's
-- ThemeRules layering: role classes at priority 0, modifiers above them.
-- Inline props still win over rules, so one-off elements just override.
function theme.rules()
	-- Text base + roles. `.text` is the left-aligned mono reset for labels;
	-- buttons stay centered (TextButton default), so they take `.button`.
	StyleRule(".text", {
		BackgroundTransparency = 1,
		Font = theme.font,
		TextColor3 = theme.text,
		TextSize = theme.textSize,
		TextXAlignment = Enum.TextXAlignment.Left,
	})
	StyleRule(".text-small", { TextSize = theme.textSizeSmall }, nil, 1)
	StyleRule(".muted", { TextColor3 = theme.textMuted }, nil, 1)
	StyleRule(".dim", { TextColor3 = theme.textDim }, nil, 1)
	StyleRule(".prop-name", { TextColor3 = theme.propName }, nil, 1)

	StyleRule(".button", { Font = theme.font, TextSize = theme.textSizeSmall })

	-- Flush header/section band (centered: no `.text` underneath).
	StyleRule(".header-band", {
		BackgroundColor3 = theme.surfaceHigh,
		BackgroundTransparency = 0,
		Font = theme.font,
		TextColor3 = theme.textMuted,
		TextSize = theme.headerSize,
	})

	-- Surfaces
	StyleRule(".surface", { BackgroundColor3 = theme.surface, BackgroundTransparency = 0 })
	StyleRule(".surface-high", { BackgroundColor3 = theme.surfaceHigh, BackgroundTransparency = 0 })
	StyleRule(".vs-border::UIStroke", {
		Thickness = 1,
		Transparency = 0,
		Color = theme.border,
		BorderStrokePosition = Enum.BorderStrokePosition.Inner,
	})

	-- Row backgrounds: the precedence waterfall as rule priorities —
	-- selected > on-path > stripe over an invisible `.row` base, so a row
	-- just carries whichever state tags apply.
	StyleRule(".row", { BackgroundColor3 = theme.bg, BackgroundTransparency = 1 })
	StyleRule(".stripe", { BackgroundColor3 = theme.bg, BackgroundTransparency = 0.5 }, nil, 1)
	StyleRule(".on-path", { BackgroundColor3 = theme.selection, BackgroundTransparency = 0.85 }, nil, 2)
	StyleRule(".selected", { BackgroundColor3 = theme.selection, BackgroundTransparency = 0 }, nil, 3)
end

return theme
