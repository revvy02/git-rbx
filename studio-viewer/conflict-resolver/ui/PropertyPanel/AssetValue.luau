local Element = require("@shared/packages").miu.Element

local format = require("@shared/library/format")
local theme = require("@shared/library/theme")

-- Visual asset id (textures/images): thumbnail preview + the id. Non-loadable
-- ids just render an empty frame — the text still carries the value.
local function AssetValue(id: string, assetId: string)
	Element("Frame", {
		id = id,
		tags = { "bg-none", "auto-size-y", "flex-row", "gap-sm" },
	}, function()
		Element("ImageLabel", {
			id = "preview",
			Image = assetId,
			ScaleType = Enum.ScaleType.Fit,
			Size = UDim2.new(0, 32, 0, 32),
			tags = { "corner-sm", "vs-border" },
			BackgroundColor3 = theme.bg,
		})
		Element("TextLabel", {
			id = "text",
			RichText = true,
			Text = format.escapeRich(assetId),
			Size = UDim2.new(1, -38, 0, 32),
			tags = { "text", "text-small", "muted" },
			TextTruncate = Enum.TextTruncate.AtEnd,
		})
	end)
end

return AssetValue
