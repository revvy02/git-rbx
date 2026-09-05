--[[
	Typed value renderer: dispatches a property value to its dedicated
	component — color swatches, checkboxes, instance refs with class icons,
	texture previews — with mono text as the fallback for everything else.
]]

local format = require("@shared/library/format")
local theme = require("@shared/library/theme")

local AssetValue = require("./AssetValue")
local BoolValue = require("./BoolValue")
local ColorValue = require("./ColorValue")
local InstanceValue = require("./InstanceValue")
local TextValue = require("./TextValue")

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
	elseif format.isVisualAsset(propertyName, value) then
		AssetValue(id, value)
	else
		TextValue(id, format.formatValue(value))
	end
end

return PropertyValue
