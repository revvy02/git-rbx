--[[
	Typed value renderer: dispatches a property value to its dedicated
	component — color swatches, checkboxes, instance refs with class icons,
	texture previews — with mono text as the fallback for everything else.
]]

local format = require("../../format")
local theme = require("../../theme")

local AssetValue = require("./AssetValue")
local BoolValue = require("./BoolValue")
local ColorValue = require("./ColorValue")
local InstanceValue = require("./InstanceValue")
local TextValue = require("./TextValue")

-- String properties whose value is a renderable image asset.
local VISUAL_ASSET_PROPS: { [string]: boolean } = {
	Texture = true,
	TextureID = true,
	TextureId = true,
	Image = true,
	Icon = true,
}

local function PropertyValue(id: string, value: any, propertyName: string?)
	local valueType = typeof(value)
	if value == nil then
		TextValue(id, "nil", theme.textDim)
	elseif valueType == "Color3" then
		ColorValue(id, value)
	elseif valueType == "boolean" then
		BoolValue(id, value)
	elseif valueType == "Instance" then
		InstanceValue(id, value)
	elseif valueType == "EnumItem" then
		TextValue(id, (value :: EnumItem).Name)
	elseif valueType == "string" and propertyName and VISUAL_ASSET_PROPS[propertyName] and value ~= "" then
		AssetValue(id, value)
	else
		TextValue(id, format.formatValue(value))
	end
end

return PropertyValue
