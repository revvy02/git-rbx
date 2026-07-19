local Element = require("../../miu/Instance")

local format = require("../../format")
local theme = require("../../theme")

-- Visual asset id (textures/images): thumbnail preview + the id. Non-loadable
-- ids just render an empty frame — the text still carries the value.
local function AssetValue(id: string, assetId: string)
	Element("Frame", {
		id = id,
		Size = UDim2.new(1, 0, 0, 0),
		AutomaticSize = Enum.AutomaticSize.Y,
		tags = { "bg-none", "flex-row", "gap-sm" },
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
			tags = { "text-base", "auto-size-y" },
			TextColor3 = theme.textMuted,
			TextXAlignment = Enum.TextXAlignment.Left,
			TextSize = theme.textSizeSmall,
			Font = theme.font,
			TextWrapped = true,
		})
	end)
end

return AssetValue
