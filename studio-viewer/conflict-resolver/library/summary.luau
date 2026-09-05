--[[
	One-glance descriptions of a conflict's sides for the conflict list and
	the comparison card. Resolver-only: these read the stamped SideInfo /
	Base components, which the diff viewer never has.
]]

local cts = require("../ecs/cts")
local format = require("@shared/library/format")

type SideInfo = cts.SideInfo
type Base = cts.Base

local summary = {}

-- What choosing this side means, one glance: the property value it sets, the
-- destination it reparents to, or whether it keeps (edited) / deletes the target.
function summary.side(kind: string, property: string?, info: SideInfo?): string?
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
	elseif kind == "ReparentTarget" then
		local path = info.destinationPath
		return path and `→ {format.escapeRich(path)}`
	elseif kind == "DeleteVsEdit" then
		-- The non-deleting side may carry an edited clone, or be a reparent out
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

-- Base-side description for the comparison card (what unpicking keeps).
function summary.base(kind: string, base: Base): string
	if kind == "Property" then
		return format.formatValue(base.value)
	elseif kind == "PropertyBundle" then
		return "keep base bundle"
	elseif kind == "Pivot" then
		return "canonical placement"
	elseif kind == "ReparentTarget" then
		local parent = base.parent
		return if parent then `→ {format.escapeRich(format.relPath(parent))}` else "→ ?"
	end
	return "keep base"
end

return summary
