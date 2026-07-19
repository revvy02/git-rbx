--[[
	Shared conflict model + staging state. A singleton: rodeo's bundling
	preserves module-cache semantics, so every requirer sees the same table.
	init.luau populates conflicts/mergedFile at boot; components read and
	stage through here. Nothing in this module touches the disk file.
]]

export type SideInfo = {
	folder: Instance?, -- the stamped side folder (a swapped-out clone lives here)
	clone: Instance?, -- this side's version (nil = this side deletes the target)
	destination: Instance?, -- MoveTarget: the parent this side moves the target to
	destinationPath: string?,
}

export type Conflict = {
	name: string,
	kind: string,
	path: string,
	property: string?,
	resolved: string?,
	target: Instance,
	entry: Instance,

	-- 3D preview state, filled by preview.boot
	ours: SideInfo?,
	theirs: SideInfo?,
	baseValue: any, -- Property: value to restore on unpick
	baseParent: Instance?, -- MoveTarget: parent to restore on unpick
	targetParts: { [BasePart]: number }?, -- DeleteVsEdit: true transparencies
	rendered: string?, -- staged state the 3D layer currently reflects
}

local state = {}

state.UNRESOLVED_COLOR = Color3.fromRGB(255, 170, 0)
state.OURS_COLOR = Color3.fromRGB(70, 160, 255)
state.THEIRS_COLOR = Color3.fromRGB(170, 90, 255)
state.CUSTOM_COLOR = Color3.fromRGB(80, 200, 160)

state.conflicts = {} :: { Conflict }
state.selected = nil :: Conflict?
state.finished = false
state.completing = false
state.completeError = nil :: string?
state.mergedFile = ""
state.rbxDiffBin = "rbx-diff"

-- Staged decisions are session-local only and freely revisable; nothing is
-- written anywhere until Complete. stage() sets an explicit state (nil =
-- unpicked/base); choose() is the toggle the side buttons use.
function state.stage(conflict: Conflict, side: string?)
	if state.completing or state.finished then
		return
	end
	conflict.resolved = side
end

function state.choose(conflict: Conflict, side: string)
	state.stage(conflict, if conflict.resolved == side then nil else side)
end

function state.statusColor(conflict: Conflict): Color3
	if conflict.resolved == "ours" then
		return state.OURS_COLOR
	elseif conflict.resolved == "theirs" then
		return state.THEIRS_COLOR
	elseif conflict.resolved == "custom" then
		return state.CUSTOM_COLOR
	end
	return state.UNRESOLVED_COLOR
end

-- The live value of the conflicted property on the (copy) target instance.
function state.liveValue(conflict: Conflict): any
	if conflict.property == "Name" then
		return conflict.target.Name
	end
	return (conflict.target :: any)[conflict.property :: string]
end

-- Encode a captured value as the plain JSON the CLI coerces by property type.
-- Returns nil for types custom resolution doesn't support.
function state.encodeValue(v: any): any
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

-- Custom = "edit the instance in Studio, that edit is the resolution".
-- Available for Property conflicts whose live value has an encodable type.
function state.canCustom(conflict: Conflict): boolean
	return conflict.kind == "Property" and state.encodeValue(state.liveValue(conflict)) ~= nil
end

return state
