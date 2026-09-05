local Element = require("@shared/packages").miu.Element

local theme = require("@shared/library/theme")

local function Label(id: string, text: string, color: Color3?, truncate: boolean?)
	Element("TextLabel", {
		id = id,
		Text = text,
		tags = { "text", "auto-size-y" },
		TextColor3 = color or theme.text,
		TextWrapped = not truncate,
		TextTruncate = if truncate then Enum.TextTruncate.AtEnd else Enum.TextTruncate.None,
	})
end

return Label
