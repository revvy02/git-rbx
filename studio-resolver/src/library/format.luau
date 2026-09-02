--[[
	Value/path presentation shared by the panel rows, comparison card, and
	explorer — pure functions over component data, no world access.
	Everything returns RichText-safe strings.
]]

local cts = require("../ecs/cts")

type SideInfo = cts.SideInfo
type Base = cts.Base

local format = {}

function format.escapeRich(s: string): string
	return (s:gsub("&", "&amp;"):gsub("<", "&lt;"):gsub(">", "&gt;"))
end

-- Roblox spatial/color values are backed by float32 components. Nine
-- significant digits are enough to distinguish every float32 value without
-- filling the conflict list with double-precision conversion noise.
local function formatFloat32(v: number): string
	return string.format("%.9g", v)
end

local function formatFloat32Components(values: { number }): string
	local formatted = table.create(#values)
	for i, value in values do
		formatted[i] = formatFloat32(value)
	end
	return table.concat(formatted, ", ")
end

function format.formatValue(v: any): string
	local t = typeof(v)
	if t == "Color3" then
		return `Color3({formatFloat32Components({ v.R, v.G, v.B })})`
	elseif t == "number" then
		-- tostring uses Luau's shortest round-trippable representation, which
		-- preserves double-backed properties as well as ordinary compact values.
		return tostring(v)
	elseif t == "Vector2" then
		return `Vector2({formatFloat32Components({ v.X, v.Y })})`
	elseif t == "Vector3" then
		return `Vector3({formatFloat32Components({ v.X, v.Y, v.Z })})`
	elseif t == "CFrame" then
		return `CFrame({formatFloat32Components({ v:GetComponents() })})`
	elseif t == "UDim" then
		return `UDim({formatFloat32(v.Scale)}, {v.Offset})`
	elseif t == "UDim2" then
		return `UDim2({formatFloat32(v.X.Scale)}, {v.X.Offset}, {formatFloat32(v.Y.Scale)}, {v.Y.Offset})`
	elseif t == "string" then
		return `"{format.escapeRich(v)}"`
	elseif t == "boolean" then
		return tostring(v)
	end
	return t
end

-- Rust's structured diff::PropertyValue, used by the direct conflict patch.
-- Keep this separate from formatValue: these values can represent properties
-- Studio cannot read (WorldPivotData, serialized mesh content, removed refs).
function format.patchValue(encoded: any): string
	if not encoded then
		return "?"
	end
	local kind = encoded.type
	local value = encoded.value or {}
	if kind == "nil" then
		return "nil"
	elseif kind == "bool" or kind == "int32" or kind == "int64" or kind == "float32" or kind == "float64" then
		return tostring(value.value)
	elseif kind == "string" then
		local text = tostring(value.value):gsub("\r", "\\r"):gsub("\n", "\\n")
		return `"{format.escapeRich(text)}"`
	elseif kind == "binary_string" then
		return `&lt;binary {value.len} bytes&gt;`
	elseif kind == "ref" then
		return `→ {format.escapeRich(tostring(value.value))}`
	elseif kind == "vector2" then
		return `Vector2({formatFloat32Components({ value.x, value.y })})`
	elseif kind == "vector3" then
		return `Vector3({formatFloat32Components({ value.x, value.y, value.z })})`
	elseif kind == "c_frame" then
		return `CFrame({formatFloat32Components(value)})`
	elseif kind == "color3" then
		return `Color3({formatFloat32Components({ value.r, value.g, value.b })})`
	elseif kind == "brick_color" then
		return `BrickColor({value.value})`
	elseif kind == "enum" then
		return `Enum({value.value})`
	elseif kind == "u_dim" then
		return `UDim({formatFloat32(value.scale)}, {value.offset})`
	elseif kind == "u_dim2" then
		return `UDim2({formatFloat32(value.x_scale)}, {value.x_offset}, {formatFloat32(value.y_scale)}, {value.y_offset})`
	elseif kind == "number_range" then
		return `NumberRange({formatFloat32(value.min)}, {formatFloat32(value.max)})`
	elseif kind == "number_sequence" or kind == "color_sequence" then
		return `{kind == "number_sequence" and "NumberSequence" or "ColorSequence"}({#value.keypoints} keypoints)`
	elseif kind == "rect" then
		return `Rect({formatFloat32Components({ value.min_x, value.min_y, value.max_x, value.max_y })})`
	elseif kind == "other" then
		return `&lt;{format.escapeRich(tostring(value.type_name))}&gt;`
	end
	return format.escapeRich(tostring(kind))
end

-- Encode a captured value as the plain JSON the CLI coerces by property type.
-- Returns nil for types custom resolution doesn't support.
function format.encodeValue(v: any): any
	local t = typeof(v)
	if t == "number" or t == "string" or t == "boolean" then
		return v
	elseif t == "Color3" then
		return { v.R, v.G, v.B }
	elseif t == "Vector3" then
		return { v.X, v.Y, v.Z }
	elseif t == "Vector2" then
		return { v.X, v.Y }
	elseif t == "CFrame" then
		return { v:GetComponents() }
	elseif t == "UDim" then
		return { v.Scale, v.Offset }
	elseif t == "UDim2" then
		return { v.X.Scale, v.X.Offset, v.Y.Scale, v.Y.Offset }
	end
	return nil
end

-- String properties whose value is a renderable image asset (thumbnail row).
local VISUAL_ASSET_PROPS: { [string]: boolean } = {
	Texture = true,
	TextureID = true,
	TextureId = true,
	Image = true,
	Icon = true,
}

function format.isVisualAsset(propertyName: string?, value: any): boolean
	return propertyName ~= nil
		and VISUAL_ASSET_PROPS[propertyName :: string] == true
		and typeof(value) == "string"
		and value ~= ""
end

-- Real merges conflict on serialized-only properties (WorldPivotData,
-- MeshData, ...) that Luau cannot touch — reads throw. Every property access
-- in the resolver goes through here; unreadable properties still stage and
-- resolve (the CLI writes them host-side), they just can't preview.
function format.readProperty(inst: Instance, property: string): (boolean, any)
	return pcall(function()
		return (inst :: any)[property]
	end)
end

local previewPrefix = "Workspace.GitRbxModelPreview."

function format.relPath(inst: Instance): string
	local full = inst:GetFullName()
	local trimmed = full:gsub("^" .. previewPrefix:gsub("%.", "%%."), "")
	return trimmed
end

-- What choosing this side means, one glance: the property value it sets, the
-- destination it moves to, or whether it keeps (edited) / deletes the target.
function format.sideSummary(kind: string, property: string?, info: SideInfo?): string?
	if not info then
		return nil
	end
	if kind == "Property" then
		local clone = info.clone
		if not clone then
			return nil
		end
		if property == "WorldPivotData" and clone:IsA("Model") then
			return format.formatValue(clone.WorldPivot)
		end
		local ok, value = format.readProperty(clone, property :: string)
		return if ok then format.formatValue(value) else "(not scriptable)"
	elseif kind == "PropertyBundle" then
		return if info.clone then "use complete serialized bundle" else nil
	elseif kind == "MoveTarget" then
		local path = info.destinationPath
		return path and `→ {format.escapeRich(path)}`
	elseif kind == "DeleteVsEdit" then
		-- The non-deleting side may carry an edited clone, or be a move out
		-- of the deleted subtree (Destination only, no clone).
		if info.deleted then
			return "delete"
		elseif info.clone then
			return "keep (edited)"
		elseif info.destinationPath then
			return `keep → {format.escapeRich(info.destinationPath)}`
		end
		return "keep"
	elseif kind == "Pivot" then
		return if info.delta then format.formatDelta(info.delta) else nil
	end
	return nil
end

-- One-glance rigid-group delta: world translation + whether it rotates.
function format.formatDelta(delta: CFrame): string
	local p = delta.Position
	local rx, ry, rz = delta:ToOrientation()
	local rotated = math.abs(rx) + math.abs(ry) + math.abs(rz) > 1e-3
	return string.format("Δ(%.1f, %.1f, %.1f)%s", p.X, p.Y, p.Z, if rotated then " + rot" else "")
end

-- Base-side description for the comparison card (what unpicking keeps).
function format.baseSummary(kind: string, base: Base): string
	if kind == "Property" then
		return format.formatValue(base.value)
	elseif kind == "PropertyBundle" then
		return "keep base bundle"
	elseif kind == "Pivot" then
		return "canonical placement"
	elseif kind == "MoveTarget" then
		local parent = base.parent
		return if parent then `→ {format.escapeRich(format.relPath(parent))}` else "→ ?"
	end
	return "keep base"
end

return format
