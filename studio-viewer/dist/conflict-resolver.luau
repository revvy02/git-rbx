
type ArgsPras__DARKLUA_TYPE_a = {}
type InMutex__DARKLUA_TYPE_b = number & { __brand: "InMutex" }

type args__DARKLUA_TYPE_c = number | string | "A-B" | "N+" | "?" | "*" | "+"

type convert2__DARKLUA_TYPE_d = (() -> () )| { () -> () } | { [string]: any } | typeof(tonumber) | typeof(stdio.ewrite)

type convert__DARKLUA_TYPE_e = convert2__DARKLUA_TYPE_d | { [(number | string)]: convert2__DARKLUA_TYPE_d }

type count__DARKLUA_TYPE_f = number | string | "A-B" | "N+" | "?" | "*" | "+"

type Name__DARKLUA_TYPE_g<T> = (self: T, name: string) -> T

type Description__DARKLUA_TYPE_h<T> = (self: T, description: string) -> T

type Default__DARKLUA_TYPE_i<T> = (self: T, default: any) -> T

type Convert__DARKLUA_TYPE_j<T> = (self: T, convert: convert__DARKLUA_TYPE_e) -> T

type Args__DARKLUA_TYPE_k<T> = (self: T, args: args__DARKLUA_TYPE_c) -> T

type Count__DARKLUA_TYPE_l<T> = (self: T, count: count__DARKLUA_TYPE_f) -> T

type Epilog__DARKLUA_TYPE_m<T> = (self: T, epilog: string) -> T

type Target__DARKLUA_TYPE_n<T> = (self: T, target: string) -> T

type Require_command__DARKLUA_TYPE_o<T> = (self: T, boolean) -> T

type Defmode__DARKLUA_TYPE_p<T> = (self: T, defmode: string) -> T

type Init__DARKLUA_TYPE_q<T> = (self: T, Init: { [(number | string)]: string }) -> T

type Action__DARKLUA_TYPE_r<T> = (
	self: T,
	Action: ((args: any?, _: any?, exceptions: any?) -> ()
)		| "store"
		| "store_true"
		| "store_false"
		| "count"
		| "append"
		| "concat"
) -> T

type Hidden__DARKLUA_TYPE_s<T> = (self: T, Hidden: boolean) -> T

type Argname__DARKLUA_TYPE_t<T> = (self: T, argname: string | { [number]: string }) -> T

type argument__DARKLUA_TYPE_u = {
	name: Name__DARKLUA_TYPE_g<argument__DARKLUA_TYPE_u>,
	description: Description__DARKLUA_TYPE_h<argument__DARKLUA_TYPE_u>,
	default: Default__DARKLUA_TYPE_i<argument__DARKLUA_TYPE_u>,
	convert: Convert__DARKLUA_TYPE_j<argument__DARKLUA_TYPE_u>,
	args: Args__DARKLUA_TYPE_k<argument__DARKLUA_TYPE_u>,
	target: Target__DARKLUA_TYPE_n<argument__DARKLUA_TYPE_u>,
	init: Init__DARKLUA_TYPE_q<argument__DARKLUA_TYPE_u>,
	action: Action__DARKLUA_TYPE_r<argument__DARKLUA_TYPE_u>,
	hidden: Hidden__DARKLUA_TYPE_s<argument__DARKLUA_TYPE_u>,
	argname: Argname__DARKLUA_TYPE_t<argument__DARKLUA_TYPE_u>,
} & InMutex__DARKLUA_TYPE_b

type option__DARKLUA_TYPE_v = {
	name: Name__DARKLUA_TYPE_g<option__DARKLUA_TYPE_v>,
	description: Description__DARKLUA_TYPE_h<option__DARKLUA_TYPE_v>,
	default: Default__DARKLUA_TYPE_i<option__DARKLUA_TYPE_v>,
	convert: Convert__DARKLUA_TYPE_j<option__DARKLUA_TYPE_v>,
	args: Args__DARKLUA_TYPE_k<option__DARKLUA_TYPE_v>,
	count: Count__DARKLUA_TYPE_l<option__DARKLUA_TYPE_v>,
	target: Target__DARKLUA_TYPE_n<option__DARKLUA_TYPE_v>,
	defmode: Defmode__DARKLUA_TYPE_p<option__DARKLUA_TYPE_v>,
	init: Init__DARKLUA_TYPE_q<option__DARKLUA_TYPE_v>,
	action: Action__DARKLUA_TYPE_r<option__DARKLUA_TYPE_v>,
	hidden: Hidden__DARKLUA_TYPE_s<option__DARKLUA_TYPE_v>,
	argname: Argname__DARKLUA_TYPE_t<option__DARKLUA_TYPE_v>,
} & InMutex__DARKLUA_TYPE_b

type flag__DARKLUA_TYPE_w = {
	name: Name__DARKLUA_TYPE_g<flag__DARKLUA_TYPE_w>,
	description: Description__DARKLUA_TYPE_h<flag__DARKLUA_TYPE_w>,
	default: Default__DARKLUA_TYPE_i<flag__DARKLUA_TYPE_w>,
	convert: Convert__DARKLUA_TYPE_j<flag__DARKLUA_TYPE_w>,
	args: Args__DARKLUA_TYPE_k<flag__DARKLUA_TYPE_w>,
	count: Count__DARKLUA_TYPE_l<flag__DARKLUA_TYPE_w>,
	target: Target__DARKLUA_TYPE_n<flag__DARKLUA_TYPE_w>,
	init: Init__DARKLUA_TYPE_q<flag__DARKLUA_TYPE_w>,
	action: Action__DARKLUA_TYPE_r<flag__DARKLUA_TYPE_w>,
	hidden: Hidden__DARKLUA_TYPE_s<flag__DARKLUA_TYPE_w>,
	argname: Argname__DARKLUA_TYPE_t<flag__DARKLUA_TYPE_w>,
} & InMutex__DARKLUA_TYPE_b

type mutex__DARKLUA_TYPE_x = {}

type command__DARKLUA_TYPE_y = Parser__DARKLUA_TYPE_B & InMutex__DARKLUA_TYPE_b

type command_target__DARKLUA_TYPE_z = Parser__DARKLUA_TYPE_B & InMutex__DARKLUA_TYPE_b

type __callPlusType__DARKLUA_TYPE_A = typeof(setmetatable(
	{} :: Parser__DARKLUA_TYPE_B,
	{} :: {
		__call: (self: any, {
			name: string?,
			description: string?,
			epilog: string?,
		}) -> Parser__DARKLUA_TYPE_B,
	}
))

type Parser__DARKLUA_TYPE_B = {
	__index: Parser__DARKLUA_TYPE_B,
	name: Name__DARKLUA_TYPE_g<Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A>,
	description: Description__DARKLUA_TYPE_h<Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A>,
	epilog: Epilog__DARKLUA_TYPE_m<Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A>,
	require_command: Require_command__DARKLUA_TYPE_o<Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A>,

	action: (self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A, fn: (...any) -> ...any) -> Parser__DARKLUA_TYPE_B,
	parse: (self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A, argv: { [number]: string }?) -> ArgsPras__DARKLUA_TYPE_a,
	pparse: (self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A, args: { [number]: string }?) -> (boolean, ArgsPras__DARKLUA_TYPE_a),
	error: (self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A, msg: string?) -> (),

	argument: (
		self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A,
		name: string,
		description: string?,
		default: any?,
		convert: convert__DARKLUA_TYPE_e?,
		args: args__DARKLUA_TYPE_c?
	) -> argument__DARKLUA_TYPE_u & InMutex__DARKLUA_TYPE_b,
	option: (
		self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A,
		name: string,
		description: string?,
		default: any?,
		convert: convert__DARKLUA_TYPE_e?,
		args: args__DARKLUA_TYPE_c?,
		count: count__DARKLUA_TYPE_f?
	) -> option__DARKLUA_TYPE_v & InMutex__DARKLUA_TYPE_b,
	flag: (
		self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A,
		name: string,
		description: string?,
		default: any?,
		convert: convert__DARKLUA_TYPE_e?,
		args: args__DARKLUA_TYPE_c?,
		count: count__DARKLUA_TYPE_f?
	) -> flag__DARKLUA_TYPE_w & InMutex__DARKLUA_TYPE_b,
	mutex: (self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A, argument_or_option: InMutex__DARKLUA_TYPE_b,...InMutex__DARKLUA_TYPE_b?) -> mutex__DARKLUA_TYPE_x,
	command: (self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A, name: string, description: string?, epilog: string?) -> command__DARKLUA_TYPE_y,
	command_target: (
		self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A,
		name: string,
		description: string?,
		epilog: string?
	) -> command_target__DARKLUA_TYPE_z,

	group: (self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A, name: string,...InMutex__DARKLUA_TYPE_b) -> Parser__DARKLUA_TYPE_B,
	help_max_width: (self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A, number) -> Parser__DARKLUA_TYPE_B,
	usage_margin: (self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A, number) -> Parser__DARKLUA_TYPE_B,
	help_usage_margin: (self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A, number) -> Parser__DARKLUA_TYPE_B,
	help_description_margin: (self: Parser__DARKLUA_TYPE_B | __callPlusType__DARKLUA_TYPE_A, number) -> Parser__DARKLUA_TYPE_B,
}

type SideInfo__DARKLUA_TYPE_C = {
	folder: Instance?, -- the stamped side folder (a swapped-out clone lives here)
	clone: Instance?, -- this side's edited version (absent for deletes and reparents)
	moveOuts: { { folder: Instance, snapshot: Instance, destination: Instance } }?, -- edited descendants escaping a commonly deleted root
	deleted: boolean?, -- this side removes the subtree (stamped Deleted attribute)
	destination: Instance?, -- the parent this side reparents the target to
	destinationPath: string?,
	delta: CFrame?, -- Pivot: canonical/base content -> this side's world placement
	pivots: { PivotApplication__DARKLUA_TYPE_L }?, -- conditional pivots selected with this side
}

type AddedInstance__DARKLUA_TYPE_D = {
	id: number,
	parent: number?,
	name: string,
	class: string,
	properties: { [string]: any }, -- serialized diff::PropertyValue per property
}

type DocumentOp__DARKLUA_TYPE_E = {
	op: string, -- add | remove | reparent | setName | setProperty
	id: number,
	-- add
	parent: number?,
	instanceCount: number?,
	subtree: { AddedInstance__DARKLUA_TYPE_D }?,
	-- reparent
	from: number?,
	to: number?,
	-- setName / setProperty
	property: string?,
	before: any?, -- setName: string; setProperty: serialized diff::PropertyValue
	after: any?,
}

type ImpactSide__DARKLUA_TYPE_F = {
	ops: { DocumentOp__DARKLUA_TYPE_E },
	affectedIds: { number },
	instanceCount: number,
	propertyCount: number,
}

type Base__DARKLUA_TYPE_G = {
	value: any, -- Property: value to restore
	parent: Instance?, -- ReparentTarget: parent to restore
	parts: { [BasePart]: number }?, -- clone swaps: true target transparencies
}

type ExplorerNode__DARKLUA_TYPE_H = {
	id: number?, -- logical identity shared by matched nodes across version trees
	source: Instance?,
	subject: Instance?,
	entity: any?,
	name: string,
	className: string?,
	phantom: boolean?,
	parent: ExplorerNode__DARKLUA_TYPE_H?,
	children: { ExplorerNode__DARKLUA_TYPE_H },
}

type ExplorerProjection__DARKLUA_TYPE_I = {
	roots: { ExplorerNode__DARKLUA_TYPE_H },
	index: { [Instance]: ExplorerNode__DARKLUA_TYPE_H },
	idIndex: { [number]: ExplorerNode__DARKLUA_TYPE_H }?,
	selectionIndex: { [number]: ExplorerNode__DARKLUA_TYPE_H }?, -- exact node or nearest surviving ancestor
}

type VersionTreeNode__DARKLUA_TYPE_J = {
	id: number,
	parent: number?,
	name: string,
	className: string,
}

type VersionTrees__DARKLUA_TYPE_K = { [string]: { VersionTreeNode__DARKLUA_TYPE_J } }

type PivotApplication__DARKLUA_TYPE_L = {
	target: Instance,
	order: number,
	parentOrder: number?,
	delta: CFrame,
}

type SideInfo__DARKLUA_TYPE_M = SideInfo__DARKLUA_TYPE_C
type ImpactSide__DARKLUA_TYPE_N = ImpactSide__DARKLUA_TYPE_F
type DocumentOp__DARKLUA_TYPE_O = DocumentOp__DARKLUA_TYPE_E
type World__DARKLUA_TYPE_P = any -- jecs.World

type Cts__DARKLUA_TYPE_Q = typeof(cts)

type Scheduler__DARKLUA_TYPE_R = any -- Planck.Scheduler

type SideInfo__DARKLUA_TYPE_S = SideInfo__DARKLUA_TYPE_C
type ImpactOperation__DARKLUA_TYPE_T = cts.ImpactOperation

type ImpactSide__DARKLUA_TYPE_U = ImpactSide__DARKLUA_TYPE_F
type Base__DARKLUA_TYPE_V = Base__DARKLUA_TYPE_G
type ExplorerNode__DARKLUA_TYPE_W = ExplorerNode__DARKLUA_TYPE_H
type ExplorerProjection__DARKLUA_TYPE_X = ExplorerProjection__DARKLUA_TYPE_I
type VersionTreeNode__DARKLUA_TYPE_Y = VersionTreeNode__DARKLUA_TYPE_J
type VersionTrees__DARKLUA_TYPE_Z = VersionTrees__DARKLUA_TYPE_K
type PivotApplication__DARKLUA_TYPE__ = PivotApplication__DARKLUA_TYPE_L
type SystemInit__DARKLUA_TYPE_0 = (world: World__DARKLUA_TYPE_P, cts: Cts__DARKLUA_TYPE_Q, scheduler: Scheduler__DARKLUA_TYPE_R) -> ()

type SideInfo__DARKLUA_TYPE_1 = SideInfo__DARKLUA_TYPE_S
type Base__DARKLUA_TYPE_2 = Base__DARKLUA_TYPE_V
type SideInfo__DARKLUA_TYPE_3 = SideInfo__DARKLUA_TYPE_S
type ExplorerNode__DARKLUA_TYPE_4 = ExplorerNode__DARKLUA_TYPE_W
type ExplorerProjection__DARKLUA_TYPE_5 = ExplorerProjection__DARKLUA_TYPE_X
type VersionTreeNode__DARKLUA_TYPE_6 = VersionTreeNode__DARKLUA_TYPE_Y
type VersionTrees__DARKLUA_TYPE_7 = VersionTrees__DARKLUA_TYPE_Z
type Options__DARKLUA_TYPE_8 = {
	fov: number?, -- field of view in degrees (default 70)
	scale: number?, -- multiplies the framing distance (default 1)
	preset: string?, -- key into `presets` (default "default")
	direction: Vector3?, -- overrides the preset direction
	aspectRatio: number?, -- viewport aspect (default 1)
	offset: Vector3?, -- added to the look-at target (default 0)
	rotation: number?, -- extra yaw in degrees around the model
}

type Size__DARKLUA_TYPE_9 = {
	width: number?,
	height: number?,
	minWidth: number?,
	minHeight: number?,
}

type ClassIcon__DARKLUA_TYPE_aa = { Image: string, ImageRectOffset: Vector2, ImageRectSize: Vector2 }

type TreeRowProps__DARKLUA_TYPE_ab = {
	id: string,
	layoutOrder: number?,
	depth: number,
	label: string, -- RichText
	className: string?, -- class icon; omit for placeholder rows
	phantom: boolean?,
	hasChildren: boolean?,
	expanded: boolean?,
	selected: boolean?,
	onPath: boolean?, -- ancestor of the selection
	color: Color3?, -- label color (conflict status / side)
	stripe: boolean?, -- zebra shading for alternating rows
	onActivated: (() -> ())?,
	onToggle: (() -> ())?,
}

type ExplorerNode__DARKLUA_TYPE_ac = ExplorerNode__DARKLUA_TYPE_H
type ExplorerProjection__DARKLUA_TYPE_ad = ExplorerProjection__DARKLUA_TYPE_I
type Row__DARKLUA_TYPE_ae = {
	node: ExplorerNode__DARKLUA_TYPE_ac,
	source: Instance?,
	subject: Instance?,
	expansionKey: any,
	label: string,
	searchText: string,
	className: string?,
	phantom: boolean?,
	depth: number,
	hasChildren: boolean,
	isExpanded: boolean,
}

type Item__DARKLUA_TYPE_af = {
	y: number,
	h: number,
	kind: "section" | "prop" | "tag",
	name: string,
	value: any,
	propertyName: string?,
	stripe: boolean,
}

type SideInfo__DARKLUA_TYPE_ag = SideInfo__DARKLUA_TYPE_C
type Base__DARKLUA_TYPE_ah = Base__DARKLUA_TYPE_G
type Id__DARKLUA_TYPE_ai = number | string

type PatchValue__DARKLUA_TYPE_aj = any

type ChangeKind__DARKLUA_TYPE_ak = "Edit" | "Reparent" | "Pivot" | "Add"

type ChangeRow__DARKLUA_TYPE_al = {
	property: string,
	kind: ChangeKind__DARKLUA_TYPE_ak,
	current: PatchValue__DARKLUA_TYPE_aj?,
	incoming: PatchValue__DARKLUA_TYPE_aj?,
	currentText: string?, -- RichText override; wins over `current`
	incomingText: string?,
	warning: boolean?,
}

type NodeKind__DARKLUA_TYPE_am = "Add" | "Remove" | "Edit" | "Reparent" | "Pivot"

type PatchNodeInput__DARKLUA_TYPE_an = {
	id: Id__DARKLUA_TYPE_ai,
	name: string,
	className: string,
	parent: Id__DARKLUA_TYPE_ai?,
	instance: Instance?,
	kind: NodeKind__DARKLUA_TYPE_am?, -- nil for an unchanged ancestor; derived from rows when absent but rows exist
	rows: { ChangeRow__DARKLUA_TYPE_al }?,
	subtreeCount: number?, -- Remove: instances in the removed subtree
	warning: boolean?,
}

type Patch__DARKLUA_TYPE_ao = {
	nodes: { [Id__DARKLUA_TYPE_ai]: PatchNodeInput__DARKLUA_TYPE_an },
}

type PatchNode__DARKLUA_TYPE_ap = {
	id: Id__DARKLUA_TYPE_ai,
	name: string,
	className: string,
	instance: Instance?,
	kind: NodeKind__DARKLUA_TYPE_am?,
	rows: { ChangeRow__DARKLUA_TYPE_al },
	subtreeCount: number?,
	warning: boolean?,
	parent: PatchNode__DARKLUA_TYPE_ap?,
	children: { PatchNode__DARKLUA_TYPE_ap },
	depth: number,
	isFinalChild: boolean,
}

type RowKind__DARKLUA_TYPE_aq = "node" | "header" | "change"

type Row__DARKLUA_TYPE_ar = {
	y: number,
	h: number,
	kind: RowKind__DARKLUA_TYPE_aq,
	index: number,
	node: PatchNode__DARKLUA_TYPE_ap,
	change: ChangeRow__DARKLUA_TYPE_al?,
	isFinalElement: boolean,
	depthsComplete: { [number]: boolean },
}

type Row__DARKLUA_TYPE_as = Row__DARKLUA_TYPE_ar
type Headers__DARKLUA_TYPE_at = { string }

type PatchNode__DARKLUA_TYPE_au = PatchNode__DARKLUA_TYPE_ap
type Row__DARKLUA_TYPE_av = Row__DARKLUA_TYPE_ar
type Id__DARKLUA_TYPE_aw = Id__DARKLUA_TYPE_ai
type Props__DARKLUA_TYPE_ax = {
	row: Row__DARKLUA_TYPE_av,
	rowHeight: number,
	expanded: boolean,
	selected: boolean,
	onToggle: () -> (),
	onSelect: (() -> ())?,
}

type Id__DARKLUA_TYPE_ay = Id__DARKLUA_TYPE_ai
type Patch__DARKLUA_TYPE_az = Patch__DARKLUA_TYPE_ao
type PatchNode__DARKLUA_TYPE_aA = PatchNode__DARKLUA_TYPE_ap
type ChangeRow__DARKLUA_TYPE_aB = ChangeRow__DARKLUA_TYPE_al
type Row__DARKLUA_TYPE_aC = Row__DARKLUA_TYPE_ar
type Patch__DARKLUA_TYPE_aD = Patch__DARKLUA_TYPE_ao
type PatchNodeInput__DARKLUA_TYPE_aE = PatchNodeInput__DARKLUA_TYPE_an
type ChangeRow__DARKLUA_TYPE_aF = ChangeRow__DARKLUA_TYPE_al
type PatchNode__DARKLUA_TYPE_aG = PatchNode__DARKLUA_TYPE_ap
type Id__DARKLUA_TYPE_aH = Id__DARKLUA_TYPE_ai
type Props__DARKLUA_TYPE_aI = {
	patch: Patch__DARKLUA_TYPE_aD,
	stateKey: any, -- identity of what is being shown; changing it collapses everything
	headers: { string }?, -- default { "Property", "Current", "Incoming" }
	selectedId: Id__DARKLUA_TYPE_ai?,
	onSelect: ((PatchNode__DARKLUA_TYPE_aG) -> ())?,
	emptyText: string?,
	Size: UDim2?,
	Position: UDim2?,
}

type ManifestNode__DARKLUA_TYPE_aJ = { id: number, parent: number?, name: string, className: string }

type Manifest__DARKLUA_TYPE_aK = { ManifestNode__DARKLUA_TYPE_aJ }

type Options__DARKLUA_TYPE_aL = {
	ops: { any }, -- cts.DocumentOp list
	old: Manifest__DARKLUA_TYPE_aK,
	new: Manifest__DARKLUA_TYPE_aK,
	-- Optional pivot rows: the boundary's id plus RichText for both columns.
	pivots: { { id: number, current: string, incoming: string } }?,
	-- Whether add rows list the added instance's properties (Rojo does).
	addProperties: boolean?,
}

type Index__DARKLUA_TYPE_aM = { byId: { [number]: ManifestNode__DARKLUA_TYPE_aJ } }

type ImpactSide__DARKLUA_TYPE_aN = ImpactSide__DARKLUA_TYPE_F
type SideInfo__DARKLUA_TYPE_aO = SideInfo__DARKLUA_TYPE_C
type VersionTrees__DARKLUA_TYPE_aP = VersionTrees__DARKLUA_TYPE_K local __DARKLUA_BUNDLE_MODULES={cache={}::any}do do local function __modImpl()--!nocheck


local process = require("@lune/process")
local stdio = require("@lune/stdio")

local function deep_update(t1, t2)
	for k, v in pairs(t2) do
		if type(v) == "table" then
			v = deep_update({}, v)
		end

		t1[k] = v
	end

	return t1
end

-- A property is a tuple {name, callback}.
-- properties.args is number of properties that can be set as arguments
-- when calling an object.
local function class(prototype, properties, parent)
	-- Class is the metatable of its instances.
	local cl = {}
	cl.__index = cl

	if parent then
		cl.__prototype = deep_update(deep_update({}, parent.__prototype), prototype)
	else
		cl.__prototype = prototype
	end

	if properties then
		local names = {}

		-- Create setter methods and fill set of property names.
		for _, property in ipairs(properties) do
			local name, callback = property[1], property[2]

			cl[name] = function(self, value)
				if not callback(self, value) then
					self["_" .. name] = value
				end

				return self
			end

			names[name] = true
		end

		function cl.__call(self, ...)
			-- When calling an object, if the first argument is a table,
			-- interpret keys as property names, else delegate arguments
			-- to corresponding setters in order.
			if type((...)) == "table" then
				for name, value in pairs((...)) do
					if names[name] then
						self[name](self, value)
					end
				end
			else
				local nargs = select("#", ...)

				for i, property in ipairs(properties) do
					if i > nargs or i > properties.args then
						break
					end

					local arg = select(i, ...)

					if arg ~= nil then
						self[property[1] ](self, arg)
					end
				end
			end

			return self
		end
	end

	-- If indexing class fails, fallback to its parent.
	local class_metatable = {}
	class_metatable.__index = parent

	function class_metatable.__call(self, ...)
		-- Calling a class returns its instance.
		-- Arguments are delegated to the instance.
		local object = deep_update({}, self.__prototype)
		setmetatable(object, self)
		return object(...)
	end

	return setmetatable(cl, class_metatable)
end

local function typecheck(name, types, value)
	for _, type_ in ipairs(types) do
		if type(value) == type_ then
			return true
		end
	end

	error(("bad property '%s' (%s expected, got %s)"):format(name, table.concat(types, " or "), type(value)))
end

local function typechecked(name, ...)
	local types = { ... }
	return {
		name,
		function(_, value)
			typecheck(name, types, value)
		end,
	}
end

local multiname = {
	"name",
	function(self, value)
		typecheck("name", { "string" }, value)

		for alias in value:gmatch("%S+") do
			self._name = self._name or alias
			table.insert(self._aliases, alias)
		end

		-- Do not set _name as with other properties.
		return true
	end,
}

local function parse_boundaries(str)
	if tonumber(str) then
		return tonumber(str), tonumber(str)
	end

	if str == "*" then
		return 0, math.huge
	end

	if str == "+" then
		return 1, math.huge
	end

	if str == "?" then
		return 0, 1
	end

	if str:match("^%d+%-%d+$") then
		local min, max = str:match("^(%d+)%-(%d+)$")
		return tonumber(min), tonumber(max)
	end

	if str:match("^%d+%+$") then
		local min = str:match("^(%d+)%+$")
		return tonumber(min), math.huge
	end

	return
end

local function boundaries(name)
	return {
		name,
		function(self, value)
			typecheck(name, { "number", "string" }, value)

			local min, max = parse_boundaries(value)

			if not min then
				error(("bad property '%s'"):format(name))
			end

			self["_min" .. name], self["_max" .. name] = min, max
		end,
	}
end

local actions = {}

local option_action = {
	"action",
	function(_, value)
		typecheck("action", { "function", "string" }, value)

		if type(value) == "string" and not actions[value] then
			error(("unknown action '%s'"):format(value))
		end
	end,
}

local option_init = {
	"init",
	function(self)
		self._has_init = true
	end,
}

local option_default = {
	"default",
	function(self, value)
		if type(value) ~= "string" then
			self._init = value
			self._has_init = true
			return true
		end
		return
	end,
}

local add_help = {
	"add_help",
	function(self, value)
		typecheck("add_help", { "boolean", "string", "table" }, value)

		if self._has_help then
			table.remove(self._options)
			self._has_help = false
		end

		if value then
			local help = self:flag():description("Show this help message and exit."):action(function()
				print(self:get_help())
				process.exit(0)
			end)

			if value ~= true then
				help = help(value)
			end

			if not help._name then
				help("-h")("--help")
			end

			self._has_help = true
		end
	end,
}

-- selene: allow(mixed_table)
local Parser = class({
	_arguments = {},
	_options = {},
	_commands = {},
	_mutexes = {},
	_groups = {},
	_require_command = true,
	_handle_options = true,
}, {
	args = 3,
	typechecked("name", "string"),
	typechecked("description", "string"),
	typechecked("epilog", "string"),
	typechecked("usage", "string"),
	typechecked("help", "string"),
	typechecked("require_command", "boolean"),
	typechecked("handle_options", "boolean"),
	typechecked("action", "function"),
	typechecked("command_target", "string"),
	typechecked("help_vertical_space", "number"),
	typechecked("usage_margin", "number"),
	typechecked("usage_max_width", "number"),
	typechecked("help_usage_margin", "number"),
	typechecked("help_description_margin", "number"),
	typechecked("help_max_width", "number"),
	add_help,
})

-- selene: allow(mixed_table)
local Command = class({
	_aliases = {},
}, {
	args = 3,
	multiname,
	typechecked("description", "string"),
	typechecked("epilog", "string"),
	typechecked("target", "string"),
	typechecked("usage", "string"),
	typechecked("help", "string"),
	typechecked("require_command", "boolean"),
	typechecked("handle_options", "boolean"),
	typechecked("action", "function"),
	typechecked("command_target", "string"),
	typechecked("help_vertical_space", "number"),
	typechecked("usage_margin", "number"),
	typechecked("usage_max_width", "number"),
	typechecked("help_usage_margin", "number"),
	typechecked("help_description_margin", "number"),
	typechecked("help_max_width", "number"),
	typechecked("hidden", "boolean"),
	add_help,
}, Parser)

-- selene: allow(mixed_table)
local Argument = class({
	_minargs = 1,
	_maxargs = 1,
	_mincount = 1,
	_maxcount = 1,
	_defmode = "unused",
	_show_default = true,
}, {
	args = 5,
	typechecked("name", "string"),
	typechecked("description", "string"),
	option_default,
	typechecked("convert", "function", "table"),
	boundaries("args"),
	typechecked("target", "string"),
	typechecked("defmode", "string"),
	typechecked("show_default", "boolean"),
	typechecked("argname", "string", "table"),
	typechecked("hidden", "boolean"),
	option_action,
	option_init,
})

-- selene: allow(mixed_table)
local Option = class({
	_aliases = {},
	_mincount = 0,
	_overwrite = true,
}, {
	args = 6,
	multiname,
	typechecked("description", "string"),
	option_default,
	typechecked("convert", "function", "table"),
	boundaries("args"),
	boundaries("count"),
	typechecked("target", "string"),
	typechecked("defmode", "string"),
	typechecked("show_default", "boolean"),
	typechecked("overwrite", "boolean"),
	typechecked("argname", "string", "table"),
	typechecked("hidden", "boolean"),
	option_action,
	option_init,
}, Argument)

function Parser:_inherit_property(name, default)
	local element = self

	while true do
		local value = element["_" .. name]

		if value ~= nil then
			return value
		end

		if not element._parent then
			return default
		end

		element = element._parent
	end
end

function Argument:_get_argument_list()
	local buf = {}
	local i = 1

	while i <= math.min(self._minargs, 3) do
		local argname = self:_get_argname(i)

		if self._default and self._defmode:find("a") then
			argname = "[" .. argname .. "]"
		end

		table.insert(buf, argname)
		i = i + 1
	end

	while i <= math.min(self._maxargs, 3) do
		table.insert(buf, "[" .. self:_get_argname(i) .. "]")
		i = i + 1

		if self._maxargs == math.huge then
			break
		end
	end

	if i < self._maxargs then
		table.insert(buf, "...")
	end

	return buf
end

function Argument:_get_usage()
	local usage = table.concat(self:_get_argument_list(), " ")

	if self._default and self._defmode:find("u") then
		if self._maxargs > 1 or (self._minargs == 1 and not self._defmode:find("a")) then
			usage = "[" .. usage .. "]"
		end
	end

	return usage
end

function actions.store_true(result, target)
	result[target] = true
end

function actions.store_false(result, target)
	result[target] = false
end

function actions.store(result, target, argument)
	result[target] = argument
end

function actions.count(result, target, _, overwrite)
	if not overwrite then
		result[target] = result[target] + 1
	end
end

function actions.append(result, target, argument, overwrite)
	result[target] = result[target] or {}
	table.insert(result[target], argument)

	if overwrite then
		table.remove(result[target], 1)
	end
end

function actions.concat(result, target, arguments, overwrite)
	if overwrite then
		error("'concat' action can't handle too many invocations")
	end

	result[target] = result[target] or {}

	for _, argument in ipairs(arguments) do
		table.insert(result[target], argument)
	end
end

function Argument:_get_action()
	local action, init

	if self._maxcount == 1 then
		if self._maxargs == 0 then
			action, init = "store_true", nil
		else
			action, init = "store", nil
		end
	else
		if self._maxargs == 0 then
			action, init = "count", 0
		else
			action, init = "append", {}
		end
	end

	if self._action then
		action = self._action
	end

	if self._has_init then
		init = self._init
	end

	if type(action) == "string" then
		action = actions[action]
	end

	return action, init
end

-- Returns placeholder for `narg`-th argument.
function Argument:_get_argname(narg)
	local argname = self._argname or self:_get_default_argname()

	if type(argname) == "table" then
		return argname[narg]
	else
		return argname
	end
end

function Argument:_get_default_argname()
	return "<" .. self._name .. ">"
end

function Option:_get_default_argname()
	return "<" .. self:_get_default_target() .. ">"
end

-- Returns labels to be shown in the help message.
function Argument:_get_label_lines()
	return { self._name }
end

function Option:_get_label_lines()
	local argument_list = self:_get_argument_list()

	if #argument_list == 0 then
		-- Don't put aliases for simple flags like `-h` on different lines.
		return { table.concat(self._aliases, ", ") }
	end

	local longest_alias_length = -1

	for _, alias in ipairs(self._aliases) do
		longest_alias_length = math.max(longest_alias_length, #alias)
	end

	local argument_list_repr = table.concat(argument_list, " ")
	local lines = {}

	for i, alias in ipairs(self._aliases) do
		local line = (" "):rep(longest_alias_length - #alias) .. alias .. " " .. argument_list_repr

		if i ~= #self._aliases then
			line = line .. ","
		end

		table.insert(lines, line)
	end

	return lines
end

function Command:_get_label_lines()
	return { table.concat(self._aliases, ", ") }
end

function Argument:_get_description()
	if self._default and self._show_default then
		if self._description then
			return ("%s (default: %s)"):format(self._description, self._default)
		else
			return ("default: %s"):format(self._default)
		end
	else
		return self._description or ""
	end
end

function Command:_get_description()
	return self._description or ""
end

function Option:_get_usage()
	local usage = self:_get_argument_list()
	table.insert(usage, 1, self._name)
	usage = table.concat(usage, " ")

	if self._mincount == 0 or self._default then
		usage = "[" .. usage .. "]"
	end

	return usage
end

function Argument:_get_default_target()
	return self._name
end

function Option:_get_default_target()
	local res

	for _, alias in ipairs(self._aliases) do
		if alias:sub(1, 1) == alias:sub(2, 2) then
			res = alias:sub(3)
			break
		end
	end

	res = res or self._name:sub(2)
	return (res:gsub("-", "_"))
end

function Option:_is_vararg()
	return self._maxargs ~= self._minargs
end

function Parser:_get_fullname()
	local parent = self._parent
	local buf = { self._name }

	while parent do
		table.insert(buf, 1, parent._name)
		parent = parent._parent
	end

	return table.concat(buf, " ")
end

function Parser:_update_charset(charset)
	charset = charset or {}

	for _, command in ipairs(self._commands) do
		command:_update_charset(charset)
	end

	for _, option in ipairs(self._options) do
		for _, alias in ipairs(option._aliases) do
			charset[alias:sub(1, 1)] = true
		end
	end

	return charset
end

function Parser:argument(...)
	local argument = Argument(...)
	table.insert(self._arguments, argument)
	return argument
end

function Parser:option(...)
	local option = Option(...)

	if self._has_help then
		-- stylua: ignore
		table.insert(self._options, (#self._options), option)
	else
		table.insert(self._options, option)
	end

	return option
end

function Parser:flag(...)
	return self:option():args(0)(...)
end

function Parser:command(...)
	local command = Command():add_help(true)(...)
	command._parent = self
	table.insert(self._commands, command)
	return command
end

function Parser:mutex(...)
	local elements = { ... }

	for i, element in ipairs(elements) do
		local mt = getmetatable(element)
		assert(mt == Option or mt == Argument, ("bad argument #%d to 'mutex' (Option or Argument expected)"):format(i))
	end

	table.insert(self._mutexes, elements)
	return self
end

function Parser:group(name, ...)
	assert(type(name) == "string", ("bad argument #1 to 'group' (string expected, got %s)"):format(type(name)))

	-- selene: allow(mixed_table)
	local group = { name = name, ... }

	for i, element in ipairs(group) do
		local mt = getmetatable(element)
		assert(
			mt == Option or mt == Argument or mt == Command,
			("bad argument #%d to 'group' (Option or Argument or Command expected)"):format(i + 1)
		)
	end

	table.insert(self._groups, group)
	return self
end

local usage_welcome = "Usage: "

function Parser:get_usage()
	if self._usage then
		return self._usage
	end

	local usage_margin = self:_inherit_property("usage_margin", #usage_welcome)
	local max_usage_width = self:_inherit_property("usage_max_width", 70)
	local lines = { usage_welcome .. self:_get_fullname() }

	local function add(s)
		if #lines[#lines] + 1 + #s <= max_usage_width then
			lines[#lines] = lines[#lines] .. " " .. s
		else
			lines[#lines + 1] = (" "):rep(usage_margin) .. s
		end
	end

	-- Normally options are before positional arguments in usage messages.
	-- However, vararg options should be after, because they can't be reliable used
	-- before a positional argument.
	-- Mutexes come into play, too, and are shown as soon as possible.
	-- Overall, output usages in the following order:
	-- 1. Mutexes that don't have positional arguments or vararg options.
	-- 2. Options that are not in any mutexes and are not vararg.
	-- 3. Positional arguments - on their own or as a part of a mutex.
	-- 4. Remaining mutexes.
	-- 5. Remaining options.

	local elements_in_mutexes = {}
	local added_elements = {}
	local added_mutexes = {}
	local argument_to_mutexes = {}

	local function add_mutex(mutex, main_argument)
		if added_mutexes[mutex] then
			return
		end

		added_mutexes[mutex] = true
		local buf = {}

		for _, element in ipairs(mutex) do
			if not element._hidden and not added_elements[element] then
				if getmetatable(element) == Option or element == main_argument then
					table.insert(buf, element:_get_usage())
					added_elements[element] = true
				end
			end
		end

		if #buf == 1 then
			add(buf[1])
		elseif #buf > 1 then
			add("(" .. table.concat(buf, " | ") .. ")")
		end
	end

	local function add_element(element)
		if not element._hidden and not added_elements[element] then
			add(element:_get_usage())
			added_elements[element] = true
		end
	end

	for _, mutex in ipairs(self._mutexes) do
		local is_vararg = false
		local has_argument = false

		for _, element in ipairs(mutex) do
			if getmetatable(element) == Option then
				if element:_is_vararg() then
					is_vararg = true
				end
			else
				has_argument = true
				argument_to_mutexes[element] = argument_to_mutexes[element] or {}
				table.insert(argument_to_mutexes[element], mutex)
			end

			elements_in_mutexes[element] = true
		end

		if not is_vararg and not has_argument then
			add_mutex(mutex)
		end
	end

	for _, option in ipairs(self._options) do
		if not elements_in_mutexes[option] and not option:_is_vararg() then
			add_element(option)
		end
	end

	-- Add usages for positional arguments, together with one mutex containing them, if they are in a mutex.
	for _, argument in ipairs(self._arguments) do
		-- Pick a mutex as a part of which to show this argument, take the first one that's still available.
		local mutex

		if elements_in_mutexes[argument] then
			for _, argument_mutex in ipairs(argument_to_mutexes[argument]) do
				if not added_mutexes[argument_mutex] then
					mutex = argument_mutex
				end
			end
		end

		if mutex then
			add_mutex(mutex, argument)
		else
			add_element(argument)
		end
	end

	for _, mutex in ipairs(self._mutexes) do
		add_mutex(mutex)
	end

	for _, option in ipairs(self._options) do
		add_element(option)
	end

	if #self._commands > 0 then
		if self._require_command then
			add("<command>")
		else
			add("[<command>]")
		end

		add("...")
	end

	return table.concat(lines, "\n")
end

local function split_lines(s)
	if s == "" then
		return {}
	end

	local lines = {}

	if s:sub(-1) ~= "\n" then
		s = s .. "\n"
	end

	for line in s:gmatch("([^\n]*)\n") do
		table.insert(lines, line)
	end

	return lines
end

local function autowrap_line(line, max_length)
	-- Algorithm for splitting lines is simple and greedy.
	local result_lines = {}

	-- Preserve original indentation of the line, put this at the beginning of each result line.
	-- If the first word looks like a list marker ('*', '+', or '-'), add spaces so that starts
	-- of the second and the following lines vertically align with the start of the second word.
	local indentation = line:match("^ *")

	if line:find("^ *[%*%+%-]") then
		indentation = indentation .. " " .. line:match("^ *[%*%+%-]( *)")
	end

	-- Parts of the last line being assembled.
	local line_parts = {}

	-- Length of the current line.
	local line_length = 0

	-- Index of the next character to consider.
	local index = 1

	while true do
		local word_start, word_finish, word = line:find("([^ ]+)", index)

		if not word_start then
			-- Ignore trailing spaces, if any.
			break
		end

		local preceding_spaces = line:sub(index, word_start - 1)
		index = word_finish + 1

		if (#line_parts == 0) or (line_length + #preceding_spaces + #word <= max_length) then
			-- Either this is the very first word or it fits as an addition to the current line, add it.
			table.insert(line_parts, preceding_spaces) -- For the very first word this adds the indentation.
			table.insert(line_parts, word)
			line_length = line_length + #preceding_spaces + #word
		else
			-- Does not fit, finish current line and put the word into a new one.
			table.insert(result_lines, table.concat(line_parts))
			line_parts = { indentation, word }
			line_length = #indentation + #word
		end
	end

	if #line_parts > 0 then
		table.insert(result_lines, table.concat(line_parts))
	end

	if #result_lines == 0 then
		-- Preserve empty lines.
		result_lines[1] = ""
	end

	return result_lines
end

-- Automatically wraps lines within given array,
-- attempting to limit line length to `max_length`.
-- Existing line splits are preserved.
local function autowrap(lines, max_length)
	local result_lines = {}

	for _, line in ipairs(lines) do
		local autowrapped_lines = autowrap_line(line, max_length)

		for _, autowrapped_line in ipairs(autowrapped_lines) do
			table.insert(result_lines, autowrapped_line)
		end
	end

	return result_lines
end

function Parser:_get_element_help(element)
	local label_lines = element:_get_label_lines()
	local description_lines = split_lines(element:_get_description())

	local result_lines = {}

	-- All label lines should have the same length (except the last one, it has no comma).
	-- If too long, start description after all the label lines.
	-- Otherwise, combine label and description lines.

	local usage_margin_len = self:_inherit_property("help_usage_margin", 3)
	local usage_margin = (" "):rep(usage_margin_len)
	local description_margin_len = self:_inherit_property("help_description_margin", 25)
	local description_margin = (" "):rep(description_margin_len)

	local help_max_width = self:_inherit_property("help_max_width")

	if help_max_width then
		local description_max_width = math.max(help_max_width - description_margin_len, 10)
		description_lines = autowrap(description_lines, description_max_width)
	end

	if #label_lines[1] >= (description_margin_len - usage_margin_len) then
		for _, label_line in ipairs(label_lines) do
			table.insert(result_lines, usage_margin .. label_line)
		end

		for _, description_line in ipairs(description_lines) do
			table.insert(result_lines, description_margin .. description_line)
		end
	else
		for i = 1, math.max(#label_lines, #description_lines) do
			local label_line = label_lines[i]
			local description_line = description_lines[i]

			local line = ""

			if label_line then
				line = usage_margin .. label_line
			end

			if description_line and description_line ~= "" then
				line = line .. (" "):rep(description_margin_len - #line) .. description_line
			end

			table.insert(result_lines, line)
		end
	end

	return table.concat(result_lines, "\n")
end

local function get_group_types(group)
	local types = {}

	for _, element in ipairs(group) do
		types[getmetatable(element)] = true
	end

	return types
end

function Parser:_add_group_help(blocks, added_elements, label, elements)
	local buf = { label }

	for _, element in ipairs(elements) do
		if not element._hidden and not added_elements[element] then
			added_elements[element] = true
			table.insert(buf, self:_get_element_help(element))
		end
	end

	if #buf > 1 then
		table.insert(blocks, table.concat(buf, ("\n"):rep(self:_inherit_property("help_vertical_space", 0) + 1)))
	end
end

function Parser:get_help()
	if self._help then
		return self._help
	end

	local blocks = { self:get_usage() }

	local help_max_width = self:_inherit_property("help_max_width")

	if self._description then
		local description = self._description

		if help_max_width then
			description = table.concat(autowrap(split_lines(description), help_max_width), "\n")
		end

		table.insert(blocks, description)
	end

	-- 1. Put groups containing arguments first, then other arguments.
	-- 2. Put remaining groups containing options, then other options.
	-- 3. Put remaining groups containing commands, then other commands.
	-- Assume that an element can't be in several groups.
	local groups_by_type = {
		[Argument] = {},
		[Option] = {},
		[Command] = {},
	}

	for _, group in ipairs(self._groups) do
		local group_types = get_group_types(group)

		for _, mt in ipairs({ Argument, Option, Command }) do
			if group_types[mt] then
				table.insert(groups_by_type[mt], group)
				break
			end
		end
	end

	local default_groups = {
		{ name = "Arguments", type = Argument, elements = self._arguments },
		{ name = "Options", type = Option, elements = self._options },
		{ name = "Commands", type = Command, elements = self._commands },
	}

	local added_elements = {}

	for _, default_group in ipairs(default_groups) do
		local type_groups = groups_by_type[default_group.type]

		for _, group in ipairs(type_groups) do
			self:_add_group_help(blocks, added_elements, group.name .. ":", group)
		end

		local default_label = default_group.name .. ":"

		if #type_groups > 0 then
			default_label = "Other " .. default_label:gsub("^.", string.lower)
		end

		self:_add_group_help(blocks, added_elements, default_label, default_group.elements)
	end

	if self._epilog then
		local epilog = self._epilog

		if help_max_width then
			epilog = table.concat(autowrap(split_lines(epilog), help_max_width), "\n")
		end

		table.insert(blocks, epilog)
	end

	return table.concat(blocks, "\n\n")
end

local function get_tip(context, wrong_name)
	local context_pool = {}
	local possible_name
	local possible_names = {}

	for name in pairs(context) do
		if type(name) == "string" then
			for i = 1, #name do
				possible_name = name:sub(1, i - 1) .. name:sub(i + 1)

				if not context_pool[possible_name] then
					context_pool[possible_name] = {}
				end

				table.insert(context_pool[possible_name], name)
			end
		end
	end

	for i = 1, #wrong_name + 1 do
		possible_name = wrong_name:sub(1, i - 1) .. wrong_name:sub(i + 1)

		if context[possible_name] then
			possible_names[possible_name] = true
		elseif context_pool[possible_name] then
			for _, name in ipairs(context_pool[possible_name]) do
				possible_names[name] = true
			end
		end
	end

	local first = next(possible_names)

	if first then
		if next(possible_names, first) then
			local possible_names_arr = {}

			for name in pairs(possible_names) do
				table.insert(possible_names_arr, "'" .. name .. "'")
			end

			table.sort(possible_names_arr)
			return "\nDid you mean one of these: " .. table.concat(possible_names_arr, " ") .. "?"
		else
			return "\nDid you mean '" .. first .. "'?"
		end
	else
		return ""
	end
end

local ElementState = class({
	invocations = 0,
})

function ElementState:__call(state, element)
	self.state = state
	self.result = state.result
	self.element = element
	self.target = element._target or element:_get_default_target()
	self.action, self.result[self.target] = element:_get_action()
	return self
end

function ElementState:error(fmt, ...)
	self.state:error(fmt, ...)
end

function ElementState:convert(argument, index)
	local converter = self.element._convert

	if converter then
		local ok, err

		if type(converter) == "function" then
			ok, err = converter(argument)
		elseif type(converter[index]) == "function" then
			ok, err = converter[index](argument)
		else
			ok = converter[argument]
		end

		if ok == nil then
			self:error(err and "%s" or "malformed argument '%s'", err or argument)
		end

		argument = ok
	end

	return argument
end

function ElementState:default(mode)
	return self.element._defmode:find(mode) and self.element._default
end

local function bound(noun, min, max, is_max)
	local res = ""

	if min ~= max then
		res = "at " .. (is_max and "most" or "least") .. " "
	end

	local number = is_max and max or min
	return res .. tostring(number) .. " " .. noun .. (number == 1 and "" or "s")
end

function ElementState:set_name(alias)
	self.name = ("%s '%s'"):format(alias and "option" or "argument", alias or self.element._name)
end

function ElementState:invoke()
	self.open = true
	self.overwrite = false

	if self.invocations >= self.element._maxcount then
		if self.element._overwrite then
			self.overwrite = true
		else
			local num_times_repr = bound("time", self.element._mincount, self.element._maxcount, true)
			self:error("%s must be used %s", self.name, num_times_repr)
		end
	else
		self.invocations = self.invocations + 1
	end

	self.args = {}

	if self.element._maxargs <= 0 then
		self:close()
	end

	return self.open
end

function ElementState:pass(argument)
	argument = self:convert(argument, #self.args + 1)
	table.insert(self.args, argument)

	if #self.args >= self.element._maxargs then
		self:close()
	end

	return self.open
end

function ElementState:complete_invocation()
	while #self.args < self.element._minargs do
		self:pass(self.element._default)
	end
end

function ElementState:close()
	if self.open then
		self.open = false

		if #self.args < self.element._minargs then
			if self:default("a") then
				self:complete_invocation()
			else
				if #self.args == 0 then
					if getmetatable(self.element) == Argument then
						self:error("missing %s", self.name)
					elseif self.element._maxargs == 1 then
						self:error("%s requires an argument", self.name)
					end
				end

				self:error("%s requires %s", self.name, bound("argument", self.element._minargs, self.element._maxargs))
			end
		end

		local args

		if self.element._maxargs == 0 then
			args = self.args[1]
		elseif self.element._maxargs == 1 then
			if self.element._minargs == 0 and self.element._mincount ~= self.element._maxcount then
				args = self.args
			else
				args = self.args[1]
			end
		else
			args = self.args
		end

		self.action(self.result, self.target, args, self.overwrite)
	end
end

local ParseState = class({
	result = {},
	options = {},
	arguments = {},
	argument_i = 1,
	element_to_mutexes = {},
	mutex_to_element_state = {},
	command_actions = {},
})

function ParseState:__call(parser, error_handler)
	self.parser = parser
	self.error_handler = error_handler
	self.charset = parser:_update_charset()
	self:switch(parser)
	return self
end

function ParseState:error(fmt, ...)
	self.error_handler(self.parser, fmt:format(...))
end

function ParseState:switch(parser)
	self.parser = parser

	if parser._action then
		table.insert(self.command_actions, { action = parser._action, name = parser._name })
	end

	for _, option in ipairs(parser._options) do
		option = ElementState(self, option)
		table.insert(self.options, option)

		for _, alias in ipairs(option.element._aliases) do
			self.options[alias] = option
		end
	end

	for _, mutex in ipairs(parser._mutexes) do
		for _, element in ipairs(mutex) do
			if not self.element_to_mutexes[element] then
				self.element_to_mutexes[element] = {}
			end

			table.insert(self.element_to_mutexes[element], mutex)
		end
	end

	for _, argument in ipairs(parser._arguments) do
		argument = ElementState(self, argument)
		table.insert(self.arguments, argument)
		argument:set_name()
		argument:invoke()
	end

	self.handle_options = parser._handle_options
	self.argument = self.arguments[self.argument_i]
	self.commands = parser._commands

	for _, command in ipairs(self.commands) do
		for _, alias in ipairs(command._aliases) do
			self.commands[alias] = command
		end
	end
end

function ParseState:get_option(name)
	local option = self.options[name]

	if not option then
		self:error("unknown option '%s'%s", name, get_tip(self.options, name))
		return
	else
		return option
	end
end

function ParseState:get_command(name)
	local command = self.commands[name]

	if not command then
		if #self.commands > 0 then
			self:error("unknown command '%s'%s", name, get_tip(self.commands, name))
			return
		else
			self:error("too many arguments")
			return
		end
	else
		return command
	end
end

function ParseState:check_mutexes(element_state)
	if self.element_to_mutexes[element_state.element] then
		for _, mutex in ipairs(self.element_to_mutexes[element_state.element]) do
			local used_element_state = self.mutex_to_element_state[mutex]

			if used_element_state and used_element_state ~= element_state then
				self:error("%s can not be used together with %s", element_state.name, used_element_state.name)
			else
				self.mutex_to_element_state[mutex] = element_state
			end
		end
	end
end

function ParseState:invoke(option, name)
	self:close()
	option:set_name(name)
	self:check_mutexes(option, name)

	if option:invoke() then
		self.option = option
	end
end

function ParseState:pass(arg)
	if self.option then
		if not self.option:pass(arg) then
			self.option = nil
		end
	elseif self.argument then
		self:check_mutexes(self.argument)

		if not self.argument:pass(arg) then
			self.argument_i = self.argument_i + 1
			self.argument = self.arguments[self.argument_i]
		end
	else
		local command = self:get_command(arg)
		self.result[command._target or command._name] = true

		if self.parser._command_target then
			self.result[self.parser._command_target] = command._name
		end

		self:switch(command)
	end
end

function ParseState:close()
	if self.option then
		self.option:close()
		self.option = nil
	end
end

function ParseState:finalize()
	self:close()

	for i = self.argument_i, #self.arguments do
		local argument = self.arguments[i]
		if #argument.args == 0 and argument:default("u") then
			argument:complete_invocation()
		else
			argument:close()
		end
	end

	if self.parser._require_command and #self.commands > 0 then
		self:error("a command is required")
	end

	for _, option in ipairs(self.options) do
		option.name = option.name or ("option '%s'"):format(option.element._name)

		if option.invocations == 0 then
			if option:default("u") then
				option:invoke()
				option:complete_invocation()
				option:close()
			end
		end

		local mincount = option.element._mincount

		if option.invocations < mincount then
			if option:default("a") then
				while option.invocations < mincount do
					option:invoke()
					option:close()
				end
			elseif option.invocations == 0 then
				self:error("missing %s", option.name)
			else
				self:error("%s must be used %s", option.name, bound("time", mincount, option.element._maxcount))
			end
		end
	end

	for i = #self.command_actions, 1, -1 do
		self.command_actions[i].action(self.result, self.command_actions[i].name)
	end
end

function ParseState:parse(args)
	for _, arg in ipairs(args) do
		local plain = true

		if self.handle_options then
			local first = arg:sub(1, 1)

			if self.charset[first] then
				if #arg > 1 then
					plain = false

					if arg:sub(2, 2) == first then
						if #arg == 2 then
							if self.options[arg] then
								local option = self:get_option(arg)
								self:invoke(option, arg)
							else
								self:close()
							end

							self.handle_options = false
						else
							local equals = arg:find("=")
							if equals then
								local name = arg:sub(1, equals - 1)
								local option = self:get_option(name)

								if option.element._maxargs <= 0 then
									self:error("option '%s' does not take arguments", name)
								end

								self:invoke(option, name)
								self:pass(arg:sub(equals + 1))
							else
								local option = self:get_option(arg)
								self:invoke(option, arg)
							end
						end
					else
						for i = 2, #arg do
							local name = first .. arg:sub(i, i)
							local option = self:get_option(name)
							self:invoke(option, name)

							if i ~= #arg and option.element._maxargs > 0 then
								self:pass(arg:sub(i + 1))
								break
							end
						end
					end
				end
			end
		end

		if plain then
			self:pass(arg)
		end
	end

	self:finalize()
	return self.result
end

function Parser:error(msg)
	stdio.ewrite(("%s\n\nError: %s\n"):format(self:get_usage(), msg))
	process.exit(1)
end

-- Compatibility with strict.lua and other checkers:
local default_cmdline = require("@lune/process").args

function Parser:_parse(args, error_handler)
	return ParseState(self, error_handler):parse(args or default_cmdline)
end

function Parser:parse(args)
	return self:_parse(args, self.error)
end

local function xpcall_error_handler(err)
	return tostring(err) .. "\noriginal " .. debug.traceback("", 2):sub(2)
end

function Parser:pparse(args)
	local parse_error

	local ok, result = xpcall(function()
		return self:_parse(args, function(_, err)
			parse_error = err
			error(err, 0)
		end)
	end, xpcall_error_handler)

	if ok then
		return true, result
	elseif not parse_error then
		error(result, 0)
	else
		return false, parse_error
	end
end

local argparse = {}

argparse.version = "0.6.0"

setmetatable(argparse, {
	__call = function(_, ...)
		return Parser(default_cmdline[0]):add_help(true)(...)
	end,
})


















































































































































return (argparse :: any) :: typeof(setmetatable(
	{} :: {
		version: string,
	},
	{} :: {
		__call: (
			self: any,
			name: string?,
			description: string?,
			epilog: string?
		) -> typeof(setmetatable(
			{} :: Parser__DARKLUA_TYPE_B,
			{} :: {
				__call: (
					self: any,
					{
						name: string?,
						description: string?,
						epilog: string?,
					}
				) -> Parser__DARKLUA_TYPE_B,
			}
		)),
	}
))
end function __DARKLUA_BUNDLE_MODULES.a():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.a if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.a=v end return v.c end end do local function __modImpl()
local module = __DARKLUA_BUNDLE_MODULES.a()
return module end function __DARKLUA_BUNDLE_MODULES.b():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.b if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.b=v end return v.c end end do local function __modImpl()-- @generated by scripts/embed-packages.mjs; do not edit.
-- 86763 bytes of roblox_packages as an rbxm, base64.

return table.concat({
	"PHJvYmxveCGJ/w0KGgoAAAIAAABVAAAAAAAAAAAAAABJTlNUKQAAAFMAAAAAAAAA8AEAAAAABgAA",
	"AEZvbGRlcgAQEAAPAgAd8AAOBAYIEAoSFhQEBAYQAhJJTlNUSwAAAC0BAAAAAAAA/wgBAAAADAAA",
	"AE1vZHVsZVNjcmlwdABFAAEAvhECAQBQBAQCBAIDAAACABMEBAAAAgAADAACAgAFCgASBC0AAAIA",
	"gAYCAgICAgICUFJPUCoBAACOAQAAAAAAAPYRAAAAAAQAAABOYW1lAQ4AAABDb250cm9sVHlwaW5n",
	"cwgSAPIDcwkAAABMaWJyYXJpZXMGAAAAJAD7KBoAAABwZXBlZWx0b3JvNDFfdWktbGFic0AyLjQu",
	"MhQAAAByZXZ2eTAyX21pdS1weEAwLjIuMBkYAAQ1AGswLjUuMB41APsEdmlydHVhbC1saXN0QDAu",
	"NC4xFiIAQndpbmQ8ABoRGgABLwAnMBwVAPAVcmV0YWluZWQtc2NvcGVAMC4xLjATAAAAdWtlbmRp",
	"b19qZWNzFgD/JTEuMCwAAAB5ZXRhbm90aGVyY2xvd25fcGxhbmNrLXJ1bnNlcnZpY2VAMC4zLjAt",
	"cmMuMiEwAAYHJQAAOgFgLnBlc2RlbwHgR2l0UmJ4UGFja2FnZXNQUk9QkwIAAHYDAAAAAAAA8xYB",
	"AAAABAAAAE5hbWUBEAAAAEFkdmFuY2VkQ29udHJvbHMRAAAADAC2Q29udmVyc2lvbgwVAFBVdGls",
	"czkAg0RhdGF0eXBlLQABOQCFUHJpbWl0aXYVAEEFAAAAMgDxEAsAAABFbnZpcm9ubWVudA0AAABT",
	"dG9yeUNyZWF0b3IpAEFUeXBlCQABMgBSBwAAAFZ/AAALAPAEdWktbGFicwgAAABQeENhbnZhcxcA",
	"cFB4U2NhbGU/AFBjdXJ2ZRQA8glwcmVzZXRzAgAAAHB4BgAAAG1pdS1weAMKAACKAAARAANRAAMW",
	"AAMnADBfcHhCADB1aV9tAJAKAAAARGltZW4EARBzOQCwVmlydHVhbExpc3QpAQcPACAuc88AABkB",
	"AF0AEnYoAF0tbGlzdGIAAEkAQ21pdV9mAAdxAAAXAeFNYXJrZG93blJ1bGVzCSgBdXlsZUxpbmsN",
	"AAAbAACcAAEaAFBTaGVldA4AQVRoZW0bAAE2AEFXaW5kDQAA7gExdGFnOgEAmwBNd2luZJMAAIQA",
	"AMkBMGV4dGYAgURlbHRhVGltTwEwRWxlswEAIgFgUG9ydGFsLQBSY2xhc3OuAVBzY29wZYEAo3Vu",
	"bW91bnRpbmdgANEOAAAAcmV0YWluZWRfJwAIEgARLRIAAJ8AQWplY3NfAFBsYW5ja1kBEXAKAMAt",
	"cnVuc2VydmljZQ+LAeBlcGVuZGVuY3lHcmFwaPQBUVBoYXNlFAJwaXBlbGluZb4AkFNjaGVkdWxl",
	"cpUAkWNvbmRpdGlvbloCEHW+AgBrAAJhAAR9AAOwAAAZAACZAQ2jAQPXARNf1wEQX9cBAHYAAC0A",
	"AFABADkAAlIAEBJ1AgG9AMBfcnVuX3NlcnZpY2VQUk9QxU0BANXZAwAAAAAA8U4BAAAABgAAAFNv",
	"dXJjZQEcBwAAdHlwZSBSZWNvcmRMaXN0PFQ+ID0geyBbc3RyaW5nXTogVCB9Cgpsb2NhbCBVdGls",
	"cyA9IHJlcXVpcmUoc2NyaXB0LlBhcmVudC4eABMpKwDxBUNyZWF0ZUJhc2VDb250cm9sID0gIQAd",
	"LhoABDIAAigA9QFzID0ge30KCmZ1bmN0aW9uGAD0Fy5DaG9vc2U8VD4obGlzdDogeyBUIH0sIGRl",
	"ZjogbnVtYmVyPyk6ZQACTQDyLDxUPgoJaWYgI2xpc3QgPD0gMCB0aGVuCgkJZXJyb3IoIlVJLUxh",
	"YnM6IEFycmF5IGdpdmVuIGluIGEgbgAiIGNJAPEBIGlzIGVtcHR5IikKCWVuZFYA02RlZiBhbmQg",
	"ZGVmID5kAAlfABVgXwDzAkRlZiBpbmRleCAoe2RlZn0pawD3DGZvciB0aGUgYXJyYXkgaXMgb3V0",
	"c2lkZSBvZhgAcXNpemUgKHvGADN9KWCDACIKCWsBBKMALT0gWwEiKCLHAPMFIiwgbGlzdFtkZWYg",
	"b3IgMV0pCgk3AHEuTGlzdCA9IAAGFQBBRGVmSa8AET3eANRvciAxCgoJcmV0dXJuFAFPCmVuZKgB",
	"AUNFbnVtSwIDqgEJXwIDsgECYAIPsQEFBK4Ail0gPT0gbmlstwEGWAEkS2W1AQRKAQR+AAHRAAVu",
	"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",
	"bm1vdW50cyBhbGx0AzFub2ScAQ9XARYTIMIAAUYMAQwACDQhNHsKCfgCAhwDAQMDDyQFaR96nAU3",
	"fy1weEAwLjKfBQU9LXB4ogUfgX4ALPIGcGVwZWVsdG9ybzQxX3VpLWxhYnNAhyIOIwYDIAANhQDx",
	"CDomAAAtLVtbIERpbWVuc2lvbnMg4oCU7xX4Lm9zYWJsZSBsYXlvdXQgc3BlY3MgZm9yIFZpcnR1",
	"YWxMaXN0CgoJUG9ydCBvZiB1bHRpbWF0ZS1saXN0J3NRABcrXgDyDE1ldGhvZHMuCglUaHJlZSBi",
	"YXNlIHR5cGVzOqgEYXNpc3RlbrsXcShuKSAgICCRAPkMYWxsIGl0ZW1zIHNhbWUgaGVpZ2h0L3dp",
	"ZHRoOAABiwwhKHU5APMK4oCUIGdyaWQgY2VsbHMKCQlnZXR0ZXIoZlgAAwIA0eKAlCBhcmJpdHJh",
	"cnkfBmVpdGVtIGQSAVIKCVBsdbEmBRYB8hF3cmFwcGVyOgoJCXdpdGhTcGFjaW5nKGlubmVyLCBu",
	"KbUA8gFkZHMgZ2FwcyBiZXR3ZWVuwwBDCl1dCloGDGsHBEwbCa0IUlVEaW1Svh4yewoJTxAB6ABy",
	"LAoJcG9zaSA8AxIAGn0oGweHAUE9Cgl7awE6OiAiaQESInEQBP4KbyB9Cgl8ICwAAAFjAIMiLCB1",
	"ZGltMnIACy0AAmMBGSKWKQHmBQOYJgEwQARpAEYpIC0+3gAKdgCBc3BhY2VkIiwPDzRpbmc0ACEs",
	"IGQBFzpUAgK1Mt8gQ29uc3RydWN0b3JziywODwIADQShAQhAAWMge30KCmbhMweAABouTgESKKIB",
	"A14NCKkABeAHQ3sKCQngHw2LASEKCeMBISA9BRIBlzkBRwUPeQACAb4cWEZpcnN05hglKGZ9AQ+G",
	"AAICsAlSc3RhdGWLBQMEHAFcDFZ0YXRlLv0CBDYSAS4ACBgAAY0CD8cACgTIAASOANEsCgkJCV9t",
	"ZWFzdXJltQChSXRlbSA9IHRydcMIEX03BgUmAQxzAA8IAQgFOAQBowIYKKECD4IBGQE+AAGDAQFC",
	"ACIgPesCD4UBCwLzAh8o8QIeD5kADwJaAAGQAAReACUgPVoDD5YACw2WBAgHAQ1WAwuMAAPNFTEK",
	"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",
	"SSgKCQk5AgFJUApapgFJAGIgPSBwb3AOAAhepgGgTwE1AAIbAiUgPScBIyhfRwEPuwAGAk0FAvS5",
	"AhtGUWFuZCBmlF8IKUYFrEYCRwEB6k8DG1IGR0YHLAEBfkICJgABEEoDGp2XCX0pCgkpIDo6UgJF",
	"CmVuZPw4AxcAJQp/fpsBVQAIGwUhLi8TAAPGavEDU2Vjb25kcyBzaW5jZSB0aGlzzTjxByBzaXRl",
	"IGxhc3QgcmFuLCBrZXB0IG8PRoMgb3duIHBlcmCTsiBub2RlIHNvCi0tDx8BQAC1ZXIgbWVhc3Vy",
	"ZXM0AJtjYWRlbmNlLgqMAtREZWx0YVRpbWUoaWQpTm8ESQIBXgAhPSCzAALBXzVpZCkSA/YAbm93",
	"ID0gb3MuY2xvY2soJUiBbm9kZS50aW0WJkZuCgkJEQBSPSBub3euAgN0AyJkdBUAISAtzQBtLnRp",
	"bWUKMgAjCgmFAAGUAgWAAilkdIEBBc0ANAqxGOA6WmNsYXNzoAYjLi8VAAVuTj9taXWmAQMCLgED",
	"VZcCJ44FZwYmLi8YAPECIikKCnR5cGUgTWl1Tm9kZTyCTZV9PiA9IG1pdS4WABI+JgABE5qxYWJs",
	"ZUluc3RhbmOgAAOVAB4uHQABwkhRRGVyaXYoSBFlYUYRc7oFYiBmb3IgYTwAIS1uFiiTaW5nbGV0",
	"b246EwIxd3JpagQBeiNxYW5jZQotLUIAcWVydHkgKG9lQYFhbCksIHBsdWl68T0gbWl1IGV4dHJh",
	"cyAoaWQvYXR0cmlidXRlcy90YWdzL2V2ZW50cykuIFNpZ25hbHMKLS0gYXJlIHJvdXRlZCBpbnRv",
	"IGBldmVudHNgpgDxGWQgd2l0aCB0aGVpciByZWFsIGhhbmRsZXIgc2lnbmF0dXJlcy4gVGibAAZG",
	"xlJuZXZlch2G8QFlcnMgcmVqZWN0IHR5cG9zyAlhd3Jvbmct/AAB0QDyNXMuCi0tCi0tIEtOT1dO",
	"IExJTUlUQVRJT04gKGx1YXUtbGFuZy9sdWF1IzI2MDgpOiB0aGUgc29sdmVyIGNoZWNrcyBhUSmC",
	"LWxpdGVyYWzaACFnddhOsWFnYWluc3QgTWl177VCPEM+IId78wIgcmVkdWNpbmcgaXQsIHNvID8A",
	"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",
	"IgWdABFzWSMDcwEFFgAMNSkClwAEaQAPBhoJAioADwkaCAOOAA8JGkT0CQotLSBzdHlsdWE6IGln",
	"bm9yZSBzdGFydJQw8gNFQ1NfRU5USVRZX01BU0sgPSABAPQDYml0MzIubHNoaWZ0KDEsIDI0RTaR",
	"RUNTX0dFTkVSdXcGNgAMMgAoMTYyAPIAUEFJUl9PRkZTRVQgPSAJZQAHAgBIMl40OJQAlUlEX0RF",
	"TEVURZIABgIAowkgICAwYjAwMDFvOAMxAG9JU19UQUcxAAgtMTAxAI9FWENMVVNJVmgAAFswYjAx",
	"MDIACSQBCAIABZQAFDDGAP8ASElfQ09NUE9ORU5UX0lElwAAAgIANDI1NpYAe2NzT25BZGQrAAxJ",
	"ADUrICD5AEJjc09u4A4INQAPMgAAGDJkAF9DaGFuZzIADRYzMgATV24dDzIADBY0MgAEUwsETwIP",
	"lgAEFzUyAAR4EAdlAA/IAAAJLAECkREHMQAPMgABFjdkAAQyAAJSOwI4AA8yAAAWODIADmIADzIA",
	"AiI5CkovPkVjc44BDzIAAQaKAiJjc1dnAiQBBAIADpYAGTH0AQWZBAIyFS8gPfoAABkx9AEFMgAF",
	"0wAPZAAAB/QBeUV4Y2x1c2lZAg8yAAAH9AFPUmVzdMgAEBQ1IQPRTlVMTF9BUlJBWSA9ICx41C5m",
	"cmVlemUoe30pIDoaLgNbAfUGTlVMTCA9IG5ld3Byb3h5KGZhbHNlCIIBAgTyBk5URVJOQUxfRVJS",
	"T1JfSU5WQUxJRN4E8Q5JRVMgPSBbWwoJWW91IHRyaWVkIHBhc3NpbmcgYbsNAg8MsmhhcyBpbnZh",
	"bGlk5BMFKQxhYXJlIGVpqg/xAXVuYWxpdmUKCW9yIG5vbi0dDTVpbmcyAHEuIFlvdSBjKBT1A2Fi",
	"bGUgREVCVUcgbW9kZSBieXkA8QJpbiB0cnVlIHRvCglqZWNzLqUMAtcqAVsUs3RyeSBkb2luZyBp",
	"mXzyCSBpbiBvcmRlciB0byBnZXQgYmV0dGVyIHgt0WlvbnMgc28KCXRoYXRgFfQTY2FuIHVuZGVy",
	"c3RhbmQgd2hhdCB3ZW50IHdyb25nLgpdXY4BCYcuAk8A9AAoY29uZGl0aW9uLCBtc2f9LxI/1ioC",
	"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",
	"AAITNB4AHzUeAAITNR4AHzYeAAITNh4AHzceAAIiN12mCC4KCdMEDzsHUAIcCUJjb2wwkgFPMF91",
	"ClMH5AJbBwJZFwifBw9SB2kDcQgH0wYJmgAHGAABsAcHEAAMbQIEIAAD6AEEIgcDIAcBzg4GdAMU",
	"MM4HBnkJAu4kEmkICQXiBwjjBwEuChIwXEECpQgKqgcFdwcP1wJkCE4BEjHuAh8x7gIjBDUDDygK",
	"AAFYAA8YAAACOAECOQAFrAkPLQAFDz8KAwU3AAHbAQUOAAGvAwM8BAUVAAQhBgafAQg9Cg/qAu8D",
	"AAIHBgMBPAYBQgMKBgMJBwofMRgDLAEjAx8xIwMHBUkKDyMDewg6AxEytQIvbDI6AyMF0AQPIwMA",
	"AVkADxgAAAI7AwM6AAbiAg97DWYPPgP/DAMgAgdaAwF4CQ9aAxIJBA0fMmwDNwF3Ax8ydwMHBVEN",
	"D3cDkgUMA0Fjb2wzNQMvbDOOA///IwM8AgeqAwEEDQ+qAyQJMxAfM7wDQgHHAx8zxwMHBYsQD8cD",
	"qQjeAxI03gMfNN4D//8/A1gCB/oDAeAQD/oDNgmUEx80DARNARcEHzQXBAcF9xMPFwTACC4EEjUu",
	"BB81LgT//1sDdAIHSgQBDBUPSgRICScXHzVcBFgBZwQfNWcEBwWVFw9nBNcIfgQSNn4EHzZ+BP//",
	"dwOQAgeaBAGIGQ+aBFoJ7BofNqwEYwG3BB82twQHFDgrFg+3BO4IzgQSN84EHzfOBP//kwOsAgfq",
	"BBI3byQN0hsOzBgOchUOyBEOzg0OhAkO6gQSCT8DA0gDD/wEbgEHBR03BwUFJy5Cb3V0cLIpFX0U",
	"AHJpZHNfbGVuzkoChxgPJQX/BgU8BQK1KwQvAQO1GQeXHgIQAgSsAg9fBf99CqgBBWsbDJAvCDJN",
	"BOIDBNIBBUsgCukBCWcgD30dCQ8/GgkPsRYJD9MSCQ+lDgkPJwoJD1kFvQGpN1UgPSA5LNsEAgdH",
	"Egn+BGZbaSAtIDjGTQHaSXFbaWRzW2ld8EgUXUAFASEqFgpdIBFlXwUC/CEBCwAC5B4BCwACeBsB",
	"CwACvBcBCwACsBMBCwACVA8BCwACqAoBCwACrAUULP4sAqIAAdg+BHwwBFUvD0wqCwM8Kw9LKgMB",
	"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",
	"iARlA34AD4kEMgMDAQ+KBAUDawAFHgUTLFFiD4wEhSNhZHFGAYcCD4kECgIINhIghxwCLwACiwQH",
	"HgkSPxgAA5cJBbwCBSkPAbdmAdUPAk0AA7pmBSkABdNYDIMPAVgABxhVA64JAtYNRG92aWRfaDQg",
	"b3KvDwOcC1IgTWFpbt4QBrIPJioqyRETco4ARHMqKjoLACcgY0hiM2x5INIABrk+A2UAIWlymgAB",
	"AhuzIGV4ZWN1dGlvbixCVmJiZWNvbWXyAGRydW50aW2fVYIuIFRoaXMgYfJNVG9uZS10BD2xc2V0",
	"dXAgbG9naWNFEkVvdXQgWVAFp94FOSkCCzkBhAEWc+AADZIQBdQIRHJlbmTqKNIod29ybGQsIHN0",
	"YXRlZhBEICAgICg9s3J1bnMgb25jZSBv5RsG4wAB9wCTICAgIGxvY2FsVgBUYWJsZXM8nRE6xIny",
	"CihUcmFuc2Zvcm0sIE1vZGVsKTpjYWNoZWQIEQVHAAt0AAJEHJ9zdWJzZXF1ZW55AAEDkwEE2QAP",
	"zAAEMSAgINcoVmlkLCB0iwBRbW9kZWyhIgEPAQK5ACVkb48ABAIAAtgAHCg6AAYsAVMgICAgZT8S",
	"AwwABg8sAuQAAiUAAlgBFE8xbwPtwwPoAQNqAXIgcmVtb3ZlHQMFjAAEXwAECAA/YGBgvBwBBcsD",
	"Dx0GDgLKAwTsAgIvADEsCglEAgONDQczRwIuAAFyPgXCYANeJwZKCgQlACUsICIKAhMABB8KCc8P",
	"BasAB9IPCHB2A6o5BW8ARU5hbWVzAARACgOxESFlZPYMAesJAcIABXgOAzoHAiAABH9BBeUgAc0w",
	"AqQAAWcmESKIGBEixh4CFwAfLk8ACwglAAJWABYKxkMFdQAC/g4DiwQDqgAC2SwCOQMCRgAPWDwD",
	"ISA9RysCIwAFcgAC6EM7cnVuEgAEUgIFtyICvwAUPdoSAykBOCwKCSJxISwKnSsCJSw4LAoJl1wi",
	"ID1qJQP7RgLLABFznQIKxD8FigBDSW5mbwciAn0LD9QoBgZHAAJnAQ/fKAsOLgAI6igBugED8Q8T",
	"KEEQDSIpDUMAAbQcAc0ATUZuIDr/OgKuX0M+Cgkp+wAEhC4zQWRkjAJDSW5mb9kHD04CEAoLJwOl",
	"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",
	"PC9yb2Jsb3g+",
})
end function __DARKLUA_BUNDLE_MODULES.c():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.c if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.c=v end return v.c end end do local function __modImpl()--[[
	Package loading without a checkout: the pesde-installed Roblox packages
	are rojo-built into an rbxm at bundle time (mise run bundle) and embedded
	here as base64. At boot the bytes go through SerializationService, which
	accepts a standard rbxm at plugin identity, and the pesde instance-path
	shims resolve natively from the imported tree. Only string requires go
	through the bundler; require(<ModuleScript>) happens at runtime.
]]


local SerializationService = game:GetService("SerializationService")
local ServerStorage = game:GetService("ServerStorage")

local blob = __DARKLUA_BUNDLE_MODULES.c()

local ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
local DECODE: { [number]: number } = {}
for index = 1, #ALPHABET do
	DECODE[string.byte(ALPHABET, index)] = index - 1
end

local function base64Decode(text: string): buffer
	local padding = 0
	if text:sub(-2) == "==" then
		padding = 2
	elseif text:sub(-1) == "=" then
		padding = 1
	end
	local size = (#text // 4) * 3 - padding
	local out = buffer.create(size)
	local offset = 0
	for i = 1, #text, 4 do
		local a, b, c, d = string.byte(text, i, i + 3)
		local va, vb = DECODE[a], DECODE[b]
		local vc = if c == 61 then 0 else DECODE[c]
		local vd = if d == 61 then 0 else DECODE[d]
		local triple = bit32.bor(bit32.lshift(va, 18), bit32.lshift(vb, 12), bit32.lshift(vc, 6), vd)
		if offset < size then
			buffer.writeu8(out, offset, bit32.band(bit32.rshift(triple, 16), 0xff))
		end
		if offset + 1 < size then
			buffer.writeu8(out, offset + 1, bit32.band(bit32.rshift(triple, 8), 0xff))
		end
		if offset + 2 < size then
			buffer.writeu8(out, offset + 2, bit32.band(triple, 0xff))
		end
		offset += 3
	end
	return out
end

local stale = ServerStorage:FindFirstChild("GitRbxPackages")
if stale then
	stale:Destroy()
end
local packagesRoot = SerializationService:DeserializeInstancesAsync(base64Decode(blob))[1]
packagesRoot.Parent = ServerStorage

local function load(name: string): any
	return require(packagesRoot:FindFirstChild(name) :: ModuleScript) :: any
end

return {
	jecs = load("jecs"),
	miu = load("miu"),
	planck = load("planck"),
	planckRunService = load("planck_run_service"),
	virtualList = load("miu_virtual_list"),
	wind = load("miu_wind"),
}
end function __DARKLUA_BUNDLE_MODULES.d():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.d if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.d=v end return v.c end end do local function __modImpl()--[[
	Launch configuration — set once by init.luau from CLI args, read by
	transport. Config, not state: nothing here changes during the session,
	so it stays a plain module instead of living in the world.
]]


local config = {
	mergedFile = "",
	gitRbxBin = "git-rbx",
}

return config
end function __DARKLUA_BUNDLE_MODULES.e():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.e if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.e=v end return v.c end end do local function __modImpl()--[[
	Locate the stamped state container a Studio front end boots from.

	Places open in Studio directly, so the container is already in the DOM.
	Model files cannot be opened as a place: the rbxm is imported into a
	preview folder under Workspace instead. The disk file stays authoritative
	either way — the import is only ever eyes and highlights.
]]


local container = {}

container.PREVIEW_NAME = "GitRbxModelPreview"

function container.find(file: string, name: string): Instance?
	local found = game:FindFirstChild(name, true)
	if found or not file:match("%.rbxmx?$") then
		return found
	end
	local roblox = require("@rodeo/roblox")
	local stale = workspace:FindFirstChild(container.PREVIEW_NAME)
	if stale then
		stale:Destroy()
	end
	local preview = Instance.new("Folder")
	preview.Name = container.PREVIEW_NAME
	for _, root in roblox.import(file) do
		root.Parent = preview
	end
	preview.Parent = workspace
	return game:FindFirstChild(name, true)
end

return container
end function __DARKLUA_BUNDLE_MODULES.f():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.f if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.f=v end return v.c end end do local function __modImpl()--[[
	Value/path presentation shared by the panel rows, comparison card, and
	explorer — pure functions over component data, no world access.
	Everything returns RichText-safe strings.
]]


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

-- One-glance rigid-group delta: world translation + whether it rotates.
function format.formatDelta(delta: CFrame): string
	local p = delta.Position
	local rx, ry, rz = delta:ToOrientation()
	local rotated = math.abs(rx) + math.abs(ry) + math.abs(rz) > 1e-3
	return string.format("Δ(%.1f, %.1f, %.1f)%s", p.X, p.Y, p.Z, if rotated then " + rot" else "")
end

return format
end function __DARKLUA_BUNDLE_MODULES.g():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.g if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.g=v end return v.c end end do local function __modImpl()--[[
	The jecs world, and nothing else — components live in ecs/cts.luau,
	behaviors in ecs/systems/. Structured after game-prototype's shared/ecs.
]]


local packages = __DARKLUA_BUNDLE_MODULES.d()

return packages.jecs.world(true)
end function __DARKLUA_BUNDLE_MODULES.h():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.h if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.h=v end return v.c end end do local function __modImpl()--[[
	Component registry — every component/tag the resolver's world uses, in
	one place, after game-prototype's ecs/cts.luau. The world is the source
	of truth for merge + session state: conflicts are entities built from the
	stamped container (ChildOf their target's mirror entity), and session
	state lives as singletons — a component set on its own entity id, read
	with world:get(cts.X, cts.X). The diff layers will grow here:
	per-instance status tags, per-side value components.
]]


local packages = __DARKLUA_BUNDLE_MODULES.d()

local world = __DARKLUA_BUNDLE_MODULES.h()

local jecs = packages.jecs





























































































local cts = {
	-- The live preview Instance an entity mirrors.
	Ref = world:component(),

	-- Mirror-derived tree data, written by the mirror system each sync:
	-- ordered child entities per node, plus singletons for the root entity
	-- list and the Instance -> entity index.
	Children = world:component(),
	Roots = world:component(),
	EntityOf = world:component(),
	MirrorGeneration = world:component(), -- bumps when a sync changes anything; readers key caches on it

	-- Explorer projections are immutable, view-scoped tree tables. Canonical
	-- mirror entities remain shared identity; per-view parentage lives in the
	-- table, avoiding an ECS entity and several relations for every visible row.
	ExplorerProjection = world:component(),
	ExplorerGeneration = world:component(),
	ExplorerBaseView = world:entity(),
	ExplorerOursView = world:entity(),
	ExplorerTheirsView = world:entity(),
	ExplorerResultView = world:entity(),
	VersionTrees = world:component(), -- complete immutable base/ours/theirs manifests
	VirtualSubjects = world:component(), -- logical id -> live result Instance when present
	VirtualSubjectIds = world:component(), -- live result Instance -> logical id

	-- Conflict entities, one per stamped container entry.
	Kind = world:component(), -- "Property" | "PropertyBundle" | "ReparentTarget" | "DeleteVsEdit" | "Pivot"
	Path = world:component(), -- display path of the contested instance
	Property = world:component(), -- Property kind: the contested property name
	Properties = world:component(), -- PropertyBundle: atomic serialized property names
	Target = world:component(), -- the contested live Instance
	Entry = world:component(), -- the stamped container entry (Name = CLI --entry)
	Order = world:component(), -- stable list position (entry-name sort)
	PivotOrder = world:component(), -- hierarchical pivot order (ancestors first)
	PivotParentOrder = world:component(), -- nearest ancestor pivot decision
	Ours = world:component(), -- SideInfo
	Theirs = world:component(), -- SideInfo
	ImpactOurs = world:component(), -- ImpactSide
	ImpactTheirs = world:component(), -- ImpactSide
	Base = world:component(), -- Base snapshot
	Resolution = world:component(), -- staged side "ours"|"theirs"|"custom"; absent = unpicked
	Rendered = world:component(), -- staged state the 3D preview currently reflects
	Highlighted = world:component(), -- staged state the 3D highlight currently reflects

	-- Rigid-group entities (one per stamped Group_N): metadata over members.
	Members = world:component(), -- member conflict entity array
	Group = world:component(), -- member -> owning group entry name
	DeltaOurs = world:component(), -- CFrame world delta this side applies
	DeltaTheirs = world:component(),

	-- Session singletons.
	Conflicts = world:component(), -- boot-sorted conflict entity array (immutable after boot)
	Cards = world:component(), -- display list: group entities + ungrouped conflicts
	Container = world:component(), -- the stamped __GitRbxConflicts container
	PivotPlan = world:component(), -- automatic local pivots deferred by a pivot conflict
	ViewTab = world:component(), -- "base" | "ours" | "theirs" | "result"
	SelectedConflict = world:component(), -- conflict entity
	SelectedInstance = world:component(), -- Instance the explorer/props focus on
	SelectedExplorerId = world:component(), -- logical selection shared by all version projections
	ExplorerRevealGeneration = world:component(), -- bumps when Select should expand + center again
	CompleteError = world:component(), -- last Complete failure message
	Completing = world:entity(), -- tag: CLI batch in flight
	Finished = world:entity(), -- tag: finalized, file written

	-- Request entities: spawned with a payload component, consumed by their
	-- system, then tagged Destroy for the end-of-frame sweep.
	FocusRequest = world:component(), -- payload: conflict entity the camera should frame
	Destroy = world:entity(), -- tag: delete this entity in the cleanup sweep
}

for name, ct in cts do
	world:set(ct, jecs.Name, name)
end

return cts
end function __DARKLUA_BUNDLE_MODULES.i():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.i if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.i=v end return v.c end end do local function __modImpl()--[[
	Stateless helpers over conflict-entity world data — staging semantics and
	the custom-resolution predicates the UI and transport share. No caches, no
	module state: the world is the source of truth, these are just the verbs.
]]


local format = __DARKLUA_BUNDLE_MODULES.g()

local cts = __DARKLUA_BUNDLE_MODULES.i()
local world = __DARKLUA_BUNDLE_MODULES.h()





local conflicts = {}

-- The boot-sorted conflict entity array (immutable after boot).
function conflicts.list(): { any }
	return (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {}
end

-- The display list: rigid-group entities + ungrouped conflicts, in order.
function conflicts.cards(): { any }
	return (world:get(cts.Cards, cts.Cards) :: { any }?) or conflicts.list()
end

-- Select a conflict without deciding or moving the camera. The logical id
-- keeps the corresponding virtual node selected across all four Explorer
-- projections; SelectedInstance continues to drive the property panel.
function conflicts.select(entity: any)
	world:set(cts.SelectedConflict, cts.SelectedConflict, entity)
	local target = world:get(entity, cts.Target) :: Instance?
	if not target then
		world:remove(cts.SelectedInstance, cts.SelectedInstance)
		world:remove(cts.SelectedExplorerId, cts.SelectedExplorerId)
		return
	end
	world:set(cts.SelectedInstance, cts.SelectedInstance, target)
	local subjectIds = (world:get(cts.VirtualSubjectIds, cts.VirtualSubjectIds) :: { [Instance]: number }?) or {}
	local id = subjectIds[target]
	if id then
		world:set(cts.SelectedExplorerId, cts.SelectedExplorerId, id)
	else
		world:remove(cts.SelectedExplorerId, cts.SelectedExplorerId)
	end
	local revealGeneration = (world:get(cts.ExplorerRevealGeneration, cts.ExplorerRevealGeneration) :: number?) or 0
	world:set(cts.ExplorerRevealGeneration, cts.ExplorerRevealGeneration, revealGeneration + 1)
end

-- Select one logical node from the direct patch while keeping the
-- owning conflict selected. Virtual nodes intentionally clear the property
-- panel but remain selectable/revealable in every version Explorer.
function conflicts.selectExplorerId(id: number)
	world:set(cts.SelectedExplorerId, cts.SelectedExplorerId, id)
	local subjects = (world:get(cts.VirtualSubjects, cts.VirtualSubjects) :: { [number]: Instance }?) or {}
	local subject = subjects[id]
	if subject then
		world:set(cts.SelectedInstance, cts.SelectedInstance, subject)
	else
		world:remove(cts.SelectedInstance, cts.SelectedInstance)
	end
	local generation = (world:get(cts.ExplorerRevealGeneration, cts.ExplorerRevealGeneration) :: number?) or 0
	world:set(cts.ExplorerRevealGeneration, cts.ExplorerRevealGeneration, generation + 1)
end

local function emptyImpact(): ImpactSide__DARKLUA_TYPE_N
	return {
		ops = {},
		affectedIds = {},
		instanceCount = 0,
		propertyCount = 0,
	}
end

-- One side's canonical decision patch. Rigid cards aggregate their member
-- patches while preserving direct operations and de-duplicating exact extent.
function conflicts.impact(entity: any, side: string): ImpactSide__DARKLUA_TYPE_N
	local component = if side == "ours" then cts.ImpactOurs else cts.ImpactTheirs
	local members = world:get(entity, cts.Members) :: { any }?
	if not members then
		return (world:get(entity, component) :: ImpactSide__DARKLUA_TYPE_N?) or emptyImpact()
	end

	local operations: { DocumentOp__DARKLUA_TYPE_O } = {}
	local affected: { [number]: boolean } = {}
	local properties: { [string]: boolean } = {}
	for _, member in members do
		local impact = (world:get(member, component) :: ImpactSide__DARKLUA_TYPE_N?) or emptyImpact()
		for _, operation in impact.ops do
			table.insert(operations, operation)
			if operation.op == "setProperty" then
				properties[`{operation.id}:{operation.property}`] = true
			elseif operation.op == "setName" then
				properties[`{operation.id}:Name`] = true
			end
		end
		for _, id in impact.affectedIds do
			affected[id] = true
		end
	end

	local affectedIds = {}
	for id in affected do
		table.insert(affectedIds, id)
	end
	table.sort(affectedIds)
	local propertyCount = 0
	for _ in properties do
		propertyCount += 1
	end
	return {
		ops = operations,
		affectedIds = affectedIds,
		instanceCount = #affectedIds,
		propertyCount = propertyCount,
	}
end

-- A group's uniform member resolution, or nil when unpicked/mixed.
function conflicts.groupResolution(entity: any): string?
	local members = (world:get(entity, cts.Members) :: { any }?) or {}
	local first = if members[1] then world:get(members[1], cts.Resolution) :: string? else nil
	if not first then
		return nil
	end
	for index = 2, #members do
		if world:get(members[index], cts.Resolution) ~= first then
			return nil
		end
	end
	return first
end

function conflicts.stageGroup(entity: any, side: string?)
	for _, member in (world:get(entity, cts.Members) :: { any }?) or {} do
		conflicts.stage(member, side)
	end
end

-- Group toggle: stage every member to the side, or unpick all when the
-- group is already uniformly on it.
function conflicts.chooseGroup(entity: any, side: string)
	conflicts.stageGroup(entity, if conflicts.groupResolution(entity) == side then nil else side)
end

-- Delete-vs-edit: which side deletes, from the stamped Deleted attributes.
-- The other side keeps — via an edited clone or a reparent out of the subtree.
function conflicts.deleteSide(entity: any): string?
	local ours = world:get(entity, cts.Ours) :: SideInfo__DARKLUA_TYPE_M?
	local theirs = world:get(entity, cts.Theirs) :: SideInfo__DARKLUA_TYPE_M?
	if ours and ours.deleted then
		return "ours"
	elseif theirs and theirs.deleted then
		return "theirs"
	end
	return nil
end

-- What the 3D preview is actually rendering for this conflict right now:
-- normally the target, but clone-backed decisions render the selected clone.
-- Focus and highlights both follow the visible payload.
function conflicts.renderedInstance(entity: any): Instance
	local target = world:get(entity, cts.Target) :: Instance
	local kind = world:get(entity, cts.Kind)
	if kind == "PropertyBundle" then
		local side = world:get(entity, cts.Resolution) :: string?
		local info = if side == "ours"
			then world:get(entity, cts.Ours) :: SideInfo__DARKLUA_TYPE_M?
			elseif side == "theirs" then world:get(entity, cts.Theirs) :: SideInfo__DARKLUA_TYPE_M?
			else nil
		return if info and info.clone then info.clone else target
	elseif kind ~= "DeleteVsEdit" then
		return target
	end
	local keepSide = if conflicts.deleteSide(entity) == "ours" then "theirs" else "ours"
	if world:get(entity, cts.Resolution) == keepSide then
		local info = world:get(entity, if keepSide == "ours" then cts.Ours else cts.Theirs) :: SideInfo__DARKLUA_TYPE_M?
		if info and info.clone then
			return info.clone :: Instance
		end
		if info and info.moveOuts and info.moveOuts[1] then
			return info.moveOuts[1].snapshot
		end
	end
	return target
end

-- Staged decisions are session-local only and freely revisable; nothing is
-- written anywhere until Complete. stage() sets an explicit state (nil =
-- unpicked/base); choose() is the toggle the side buttons use.
function conflicts.stage(entity: any, side: string?)
	if world:has(cts.Completing, cts.Completing) or world:has(cts.Finished, cts.Finished) then
		return
	end
	if side then
		world:set(entity, cts.Resolution, side)
	else
		world:remove(entity, cts.Resolution)
	end
end

function conflicts.choose(entity: any, side: string)
	conflicts.stage(entity, if world:get(entity, cts.Resolution) == side then nil else side)
end

-- The live value of the conflicted property on the (copy) target instance.
-- nil when the property is serialized-only and Luau can't read it.
function conflicts.liveValue(entity: any): any
	local property = world:get(entity, cts.Property) :: string
	local target = world:get(entity, cts.Target) :: Instance
	if property == "Name" then
		return target.Name
	end
	local ok, value = format.readProperty(target, property)
	return if ok then value else nil
end

-- Custom = "edit the instance in Studio, that edit is the resolution".
-- Available for Property conflicts whose live value has an encodable type.
function conflicts.canCustom(entity: any): boolean
	return world:get(entity, cts.Kind) == "Property" and format.encodeValue(conflicts.liveValue(entity)) ~= nil
end

return conflicts
end function __DARKLUA_BUNDLE_MODULES.j():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.j if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.j=v end return v.c end end do local function __modImpl()--[[
	Decision transport: the canonical CLI, host-side. Complete batches every
	staged decision through `git rbx resolve` and finalizes; the disk file is
	only ever written here. Progress/outcome land in the world as the
	Completing/Finished tags and the CompleteError singleton.
]]


local HttpService = game:GetService("HttpService")

local process = require("@rodeo/process")

local config = __DARKLUA_BUNDLE_MODULES.e()
local format = __DARKLUA_BUNDLE_MODULES.g()

local conflicts = __DARKLUA_BUNDLE_MODULES.j()
local cts = __DARKLUA_BUNDLE_MODULES.i()
local world = __DARKLUA_BUNDLE_MODULES.h()

local transport = {}

-- Commit every staged decision through the canonical CLI, then finalize.
function transport.complete()
	if world:has(cts.Completing, cts.Completing) or world:has(cts.Finished, cts.Finished) then
		return
	end
	world:add(cts.Completing, cts.Completing)
	world:remove(cts.CompleteError, cts.CompleteError)
	task.spawn(function()
		for _, entity in conflicts.list() do
			local entryName = (world:get(entity, cts.Entry) :: Instance).Name
			local resolution = world:get(entity, cts.Resolution) :: string?
			local args
			if resolution == "custom" then
				-- Capture the live value NOW — the final state of the user's
				-- in-Studio edit is the resolution
				local encoded = format.encodeValue(conflicts.liveValue(entity))
				args = {
					config.gitRbxBin,
					"resolve",
					config.mergedFile,
					"--take",
					"custom",
					"--entry",
					entryName,
					"--value",
					HttpService:JSONEncode(encoded),
				}
			else
				args = {
					config.gitRbxBin,
					"resolve",
					config.mergedFile,
					"--take",
					resolution :: string,
					"--entry",
					entryName,
				}
			end
			local ok, result = pcall(process.run, args)
			if not ok or not result.ok then
				local message = `--take {entryName} failed: {if ok then result.err else result}`
				world:set(cts.CompleteError, cts.CompleteError, message)
				warn(message)
				world:remove(cts.Completing, cts.Completing)
				return
			end
		end
		local ok, result = pcall(process.run, { config.gitRbxBin, "resolve", config.mergedFile, "--finalize" })
		if ok and result.ok then
			world:add(cts.Finished, cts.Finished)
		else
			local message = `finalize failed: {if ok then result.err else result}`
			world:set(cts.CompleteError, cts.CompleteError, message)
			warn(message)
		end
		world:remove(cts.Completing, cts.Completing)
	end)
end

return transport
end function __DARKLUA_BUNDLE_MODULES.k():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.k if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.k=v end return v.c end end do local function __modImpl()--[[
	The planck scheduler over the world, RunService-phased — after
	game-prototype's ecs/scheduler.luau (minus their bench/jabby plugins).
	Systems register in init.luau and run as soon as they're added
	(stock planck; game-prototype's pause-until-start comes from their patch).
]]


local packages = __DARKLUA_BUNDLE_MODULES.d()

local world = __DARKLUA_BUNDLE_MODULES.h()

local Planck = packages.planck
local PlanckRunService = packages.planckRunService

local scheduler = Planck.Scheduler.new(world):addPlugin(PlanckRunService.Plugin.new())

return scheduler
end function __DARKLUA_BUNDLE_MODULES.l():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.l if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.l=v end return v.c end end do local function __modImpl()--[[
	Arg types for system init functions, after game-prototype's ecs/types.luau.
	The instance-require package loader erases jecs/planck types, so World and
	Scheduler are opaque; Cts keeps its keyed shape.
]]


local cts = __DARKLUA_BUNDLE_MODULES.i()

















return nil
end function __DARKLUA_BUNDLE_MODULES.m():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.m if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.m=v end return v.c end end do local function __modImpl()--[[
	Staged-selection preview. Boot stashes the stamped container out of the
	scene and fills each conflict entity's side info (Ours/Theirs folders) and
	Base snapshot (what unpicking restores); the scheduled loop renders the
	staged selection into the scene — staging Ours renders ours' version,
	Theirs renders theirs', unpicked renders base. Property values apply to
	the target, reparents relocate it, and decisions that require serialized-only
	state swap stamped clones in and out. None of this touches the disk file —
	the CLI at Complete is the
	only writer. Staged Resolution vs Rendered drives the frame diff.
]]


local packages = __DARKLUA_BUNDLE_MODULES.d()

local format = __DARKLUA_BUNDLE_MODULES.g()
local types = __DARKLUA_BUNDLE_MODULES.m()




return function(world: World__DARKLUA_TYPE_P, cts: Cts__DARKLUA_TYPE_Q, scheduler: Scheduler__DARKLUA_TYPE_R)
	local conflictList = (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {}
	local automaticPivotPlan = (world:get(cts.PivotPlan, cts.PivotPlan) :: { PivotApplication__DARKLUA_TYPE__}?) or {}

	local function eachPart(root: Instance, fn: (BasePart) -> ())
		if root:IsA("BasePart") then
			fn(root)
		end
		for _, descendant in root:GetDescendants() do
			if descendant:IsA("BasePart") then
				fn(descendant)
			end
		end
	end

	local function eachModel(root: Instance, fn: (Model) -> ())
		if root:IsA("Model") then
			fn(root)
		end
		for _, descendant in root:GetDescendants() do
			if descendant:IsA("Model") then
				fn(descendant)
			end
		end
	end

	local function capturePartTransparencies(root: Instance): { [BasePart]: number }
		local parts: { [BasePart]: number } = {}
		eachPart(root, function(part)
			parts[part] = part.Transparency
		end)
		return parts
	end

	local function sideInfo(entry: Instance, sideName: string): SideInfo__DARKLUA_TYPE_1
		local info: SideInfo__DARKLUA_TYPE_1 = {}
		local folder = entry:FindFirstChild(sideName)
		if folder then
			info.folder = folder
			for _, child in folder:GetChildren() do
				if child.Name == "Destination" and child:IsA("ObjectValue") then
					info.destination = child.Value
				elseif child.Name == "ReparentedOut" then
					local moveOuts = {}
					for _, record in child:GetChildren() do
						if record.Name == "References" then
							continue
						end
						local destinationValue = record:FindFirstChild("Destination")
						local destination = if destinationValue and destinationValue:IsA("ObjectValue")
							then destinationValue.Value
							else nil
						local snapshot = nil
						for _, candidate in record:GetChildren() do
							if not candidate:IsA("ObjectValue") then
								snapshot = candidate
								break
							end
						end
						if destination and snapshot then
							table.insert(moveOuts, {
								folder = record,
								snapshot = snapshot,
								destination = destination,
							})
						end
					end
					info.moveOuts = moveOuts
				elseif child.Name == "PivotPlan" then
					local pivots: { PivotApplication__DARKLUA_TYPE__} = {}
					for _, pivot in child:GetChildren() do
						local targetValue = pivot:FindFirstChild("Target")
						local target = if targetValue and targetValue:IsA("ObjectValue") then targetValue.Value else nil
						local order = pivot:GetAttribute("PivotOrder") :: number?
						local parentOrder = pivot:GetAttribute("PivotParentOrder") :: number?
						local delta = pivot:GetAttribute("Delta") :: CFrame?
						if target and order and delta then
							table.insert(pivots, {
								target = target,
								order = order,
								parentOrder = parentOrder,
								delta = delta,
							})
						end
					end
					info.pivots = pivots
				elseif child.Name ~= "__GitRbxImpact" then
					info.clone = child
				end
			end
			info.deleted = folder:GetAttribute("Deleted") :: boolean?
			info.destinationPath = folder:GetAttribute("DestinationPath") :: string?
			info.delta = folder:GetAttribute("Delta") :: CFrame?
		end
		return info
	end

	-- Boot pass: the stamped container rides inside model files, so a model
	-- preview imports it into workspace — where every clone renders on top
	-- of its target. It is pure data for this session: stash it out of the
	-- scene (copy-only; the file keeps its container until finalize strips
	-- it), then per conflict entity set side clones + the base snapshot.
	local container = world:get(cts.Container, cts.Container) :: Instance
	if container:IsDescendantOf(workspace) then
		container.Parent = game:GetService("ServerStorage")
	end

	-- Capture one canonical snapshot across every automatic and selectable
	-- frame boundary. Overlapping boundaries intentionally share entries in
	-- these maps: restoration writes the same canonical value exactly once.
	local canonicalFrameParts: { [BasePart]: CFrame } = {}
	local canonicalFrameModels: { [Model]: CFrame } = {}
	local function captureFrameTarget(target: Instance)
		eachPart(target, function(part)
			canonicalFrameParts[part] = canonicalFrameParts[part] or part.CFrame
		end)
		eachModel(target, function(model)
			canonicalFrameModels[model] = canonicalFrameModels[model] or model.WorldPivot
		end)
	end
	for _, application in automaticPivotPlan do
		captureFrameTarget(application.target)
	end

	for _, entity in conflictList do
		local entry = world:get(entity, cts.Entry) :: Instance
		local ours = sideInfo(entry, "Ours")
		local theirs = sideInfo(entry, "Theirs")
		world:set(entity, cts.Ours, ours)
		world:set(entity, cts.Theirs, theirs)
		world:set(entity, cts.Rendered, "__init")
		for _, info in { ours, theirs } do
			for _, application in info.pivots or {} do
				captureFrameTarget(application.target)
			end
		end

		local target = world:get(entity, cts.Target) :: Instance
		local kind = world:get(entity, cts.Kind)
		local base: Base__DARKLUA_TYPE_2 = {}
		if kind == "Property" then
			local property = world:get(entity, cts.Property) :: string
			if property == "Name" then
				base.value = target.Name
			elseif property == "WorldPivotData" and target:IsA("Model") then
				base.value = target.WorldPivot
			else
				local ok, value = format.readProperty(target, property)
				base.value = if ok then value else nil
			end
		elseif kind == "PropertyBundle" or kind == "DeleteVsEdit" then
			-- Both kinds use clone swaps. Mesh geometry includes serialized-only
			-- InitialSize, so property writes cannot faithfully preview its bundle.
			base.parts = capturePartTransparencies(target)
		elseif kind == "ReparentTarget" then
			base.parent = target.Parent
		elseif kind == "Pivot" then
			captureFrameTarget(target)
		end
		world:set(entity, cts.Base, base)
	end

	-- Render the staged selection; nil (unpicked) restores the base snapshot.
	local function applyPreview(entity: any)
		local staged = world:get(entity, cts.Resolution) :: string?
		local kind = world:get(entity, cts.Kind)
		local target = world:get(entity, cts.Target) :: Instance
		local ours = world:get(entity, cts.Ours) :: SideInfo__DARKLUA_TYPE_1
		local theirs = world:get(entity, cts.Theirs) :: SideInfo__DARKLUA_TYPE_1
		local base = world:get(entity, cts.Base) :: Base__DARKLUA_TYPE_2

		if kind == "Property" then
			if staged ~= "custom" then
				local sideClone = if staged == "ours"
					then ours.clone
					elseif staged == "theirs" then theirs.clone
					else nil
				local property = world:get(entity, cts.Property) :: string
				if property == "WorldPivotData" and target:IsA("Model") then
					-- WorldPivotData itself is serialized-only. WorldPivot exposes
					-- the same pivot and, unlike PivotTo, assigning it deliberately
					-- does not move descendants.
					if sideClone and sideClone:IsA("Model") then
						target.WorldPivot = sideClone.WorldPivot
					elseif staged == nil and base.value ~= nil then
						target.WorldPivot = base.value
					end
				elseif sideClone then
					pcall(function()
						(target :: any)[property] = (sideClone :: any)[property]
					end)
				elseif staged == nil and base.value ~= nil then
					pcall(function()
						(target :: any)[property] = base.value
					end)
				end
			end
		elseif kind == "PropertyBundle" then
			local selected: SideInfo__DARKLUA_TYPE_1? = if staged == "ours" then ours elseif staged == "theirs" then theirs else nil
			local function placeClone(side: string, info: SideInfo__DARKLUA_TYPE_1)
				local clone = info.clone
				if clone then
					-- Preserve independently staged placement/scale while borrowing the
					-- clone's inseparable mesh asset + InitialSize payload.
					if clone:IsA("BasePart") and target:IsA("BasePart") then
						clone.CFrame = target.CFrame
						clone.Size = target.Size
					end
					clone.Parent = if staged == side then target.Parent else info.folder
				end
			end
			placeClone("ours", ours)
			placeClone("theirs", theirs)
			local hideTarget = selected ~= nil and selected.clone ~= nil
			for part, original in base.parts :: { [BasePart]: number } do
				part.Transparency = if hideTarget then 1 else original
			end
		elseif kind == "ReparentTarget" then
			local destination = if staged == "ours"
				then ours.destination
				elseif staged == "theirs" then theirs.destination
				else nil
			-- A side with no live destination (reparented into the other branch's added
			-- content) previews as base rather than sticking at a stale parent.
			destination = destination or base.parent
			if destination then
				target.Parent = destination
			end
		elseif kind == "DeleteVsEdit" then
			-- One side deletes (stamped Deleted attribute); the other keeps —
			-- an edited clone, or a reparent out of the deleted subtree (no clone).
			-- Staging the delete side hides the base target; staging a keep
			-- side with an edited clone swaps the clone into the target's spot
			-- (it carries its own true coordinates). A clone-less keep side
			-- just keeps the target visible.
			local deleteSide = if ours.deleted then "ours" elseif theirs.deleted then "theirs" else nil
			local keepSide = if deleteSide == "ours" then "theirs" else "ours"
			local keepInfo = if keepSide == "ours" then ours else theirs
			local clone = keepInfo.clone
			local moveOuts = keepInfo.moveOuts or {}
			local hideTarget = staged == deleteSide or ((clone ~= nil or #moveOuts > 0) and staged == keepSide)
			for part, original in base.parts :: { [BasePart]: number } do
				part.Transparency = if hideTarget then 1 else original
			end
			if clone then
				clone.Parent = if staged == keepSide then target.Parent else keepInfo.folder
			end
			for _, moveOut in moveOuts do
				moveOut.snapshot.Parent = if staged == keepSide then moveOut.destination else moveOut.folder
			end
		end
	end

	local function restoreCanonicalFrames()
		for part, cframe in canonicalFrameParts do
			if part.Parent then
				part.CFrame = cframe
			end
		end
		for model, pivot in canonicalFrameModels do
			if model.Parent then
				model.WorldPivot = pivot
			end
		end
	end

	local function applyPivotPlan()
		local applications: { PivotApplication__DARKLUA_TYPE__} = table.clone(automaticPivotPlan)
		for _, entity in conflictList do
			local staged = world:get(entity, cts.Resolution) :: string?
			local info = if staged == "ours"
				then world:get(entity, cts.Ours) :: SideInfo__DARKLUA_TYPE_1
				elseif staged == "theirs" then world:get(entity, cts.Theirs) :: SideInfo__DARKLUA_TYPE_1
				else nil
			if info then
				if world:get(entity, cts.Kind) == "Pivot" and info.delta then
					table.insert(applications, {
						target = world:get(entity, cts.Target) :: Instance,
						order = (world:get(entity, cts.PivotOrder) :: number?) or math.huge,
						parentOrder = world:get(entity, cts.PivotParentOrder) :: number?,
						delta = info.delta,
					})
				end
				for _, pivot in info.pivots or {} do
					table.insert(applications, pivot)
				end
			end
		end
		table.sort(applications, function(a, b)
			return a.order < b.order
		end)

		-- Compose every local choice first. Direct property writes then visit
		-- each descendant once, avoiding both compounded preview transforms and
		-- float drift from repeated parent/child PivotTo calls.
		local effectiveByOrder: { [number]: CFrame } = {}
		local effectiveByTarget: { [Instance]: CFrame } = {}
		local roots: { Instance } = {}
		for _, application in applications do
			local parent = if application.parentOrder then effectiveByOrder[application.parentOrder] else nil
			local effective = application.delta * (parent or CFrame.identity)
			effectiveByOrder[application.order] = effective
			effectiveByTarget[application.target] = effective
			if parent == nil then
				table.insert(roots, application.target)
			end
		end

		local function visit(instance: Instance, inherited: CFrame?)
			local effective = effectiveByTarget[instance] or inherited
			if effective then
				if instance:IsA("BasePart") then
					instance.CFrame = effective * instance.CFrame
				elseif instance:IsA("Model") then
					-- Assigning WorldPivot directly changes only the serialized pivot;
					-- descendants receive their own one-time world transforms below.
					instance.WorldPivot = effective * instance.WorldPivot
				end
			end
			for _, child in instance:GetChildren() do
				visit(child, effective)
			end
		end
		for _, root in roots do
			if root.Parent then
				visit(root, nil)
			end
		end
	end

	scheduler:addSystem({
		name = "render_previews",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			if world:has(cts.Finished, cts.Finished) then
				return
			end
			debug.profilebegin("gitrbx_previews")
			local changed = false
			for _, entity in conflictList do
				local staged = (world:get(entity, cts.Resolution) :: string?) or "unresolved"
				if world:get(entity, cts.Rendered) ~= staged then
					changed = true
					break
				end
			end
			if changed then
				-- Rebuild from the canonical frame on every decision change so
				-- switching ours/theirs never compounds transforms. Ordinary
				-- property/tree choices run first; the complete hierarchical frame
				-- plan then runs once in parent-before-child order.
				restoreCanonicalFrames()
				for _, entity in conflictList do
					if world:get(entity, cts.Kind) ~= "Pivot" then
						applyPreview(entity)
					end
				end
				applyPivotPlan()
				for _, entity in conflictList do
					local staged = (world:get(entity, cts.Resolution) :: string?) or "unresolved"
					world:set(entity, cts.Rendered, staged)
				end
			end
			debug.profileend()
		end,
	})
end
end function __DARKLUA_BUNDLE_MODULES.n():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.n if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.n=v end return v.c end end do local function __modImpl()--[[
	Instance mirror: preview instances become entities, synced by frame diff —
	walk the DOM, ensure entities and exclusive ChildOf pairs, delete entities
	whose instance left the game. Session artifacts (Highlights, the stamped
	container) stay out of the mirror.

	The tree itself is world data, not an API: each node carries a Children
	component (child entities, name-ordered), the root entity list is the
	Roots singleton, and the Instance -> entity index is the EntityOf
	singleton. Conflict entities get parented (ChildOf) under their target's
	mirror entity here too — readers walk components, nothing calls into this
	module.
]]


local packages = __DARKLUA_BUNDLE_MODULES.d()

local types = __DARKLUA_BUNDLE_MODULES.m()

local jecs = packages.jecs
local pair = jecs.pair
local ChildOf = jecs.ChildOf

return function(world: World__DARKLUA_TYPE_P, cts: Cts__DARKLUA_TYPE_Q, scheduler: Scheduler__DARKLUA_TYPE_R)
	-- System-private Instance -> entity map, published as the EntityOf
	-- singleton (same table, mutated in place).
	local entities: { [Instance]: any } = {}
	world:set(cts.EntityOf, cts.EntityOf, entities)

	-- Anything a sync changes bumps MirrorGeneration, so readers (explorer
	-- row cache) can key derived data on it instead of re-walking per frame.
	local generation = 0
	local dirty = false

	local function entityFor(inst: Instance): any
		local entity = entities[inst]
		if not entity then
			entity = world:entity()
			world:set(entity, cts.Ref, inst)
			world:set(entity, jecs.Name, inst.Name)
			entities[inst] = entity
			dirty = true
		end
		return entity
	end

	local targets = world:query(cts.Target):cached()

	-- Model previews root at the imported content; places root at Workspace
	-- plus any service holding a conflict target.
	local function treeRoots(): { Instance }
		local previewFolder = workspace:FindFirstChild("GitRbxModelPreview")
		if previewFolder then
			return previewFolder:GetChildren()
		end
		local roots: { Instance } = { workspace }
		local seen: { [Instance]: boolean } = { [workspace :: Instance] = true }
		for _, target in targets do
			local top: Instance = target :: Instance
			while top.Parent and top.Parent ~= game do
				top = top.Parent :: Instance
			end
			if not seen[top] then
				seen[top] = true
				table.insert(roots, top)
			end
		end
		return roots
	end

	local function sameChildren(old: { any }?, new: { any }): boolean
		if old == nil or #old ~= #new then
			return false
		end
		for index, entity in new do
			if (old :: { any })[index] ~= entity then
				return false
			end
		end
		return true
	end

	local function syncTree(roots: { Instance })
		local seen: { [Instance]: boolean } = {}

		local function visit(inst: Instance, parentEntity: any?): any
			seen[inst] = true
			local entity = entityFor(inst)
			if parentEntity then
				if world:parent(entity) ~= parentEntity then
					world:add(entity, pair(ChildOf, parentEntity))
					dirty = true
				end
			end
			-- Renames must invalidate derived labels.
			if world:get(entity, jecs.Name) ~= inst.Name then
				world:set(entity, jecs.Name, inst.Name)
				dirty = true
			end

			local kids: { Instance } = {}
			for _, child in inst:GetChildren() do
				if child.ClassName ~= "Highlight" and child.Name ~= "__GitRbxConflicts" then
					table.insert(kids, child)
				end
			end
			table.sort(kids, function(a: Instance, b: Instance)
				return a.Name < b.Name
			end)

			local childEntities = table.create(#kids)
			for index, child in kids do
				childEntities[index] = visit(child, entity)
			end
			if not sameChildren(world:get(entity, cts.Children), childEntities) then
				world:set(entity, cts.Children, childEntities)
				dirty = true
			end
			return entity
		end

		local rootEntities = table.create(#roots)
		for index, root in roots do
			rootEntities[index] = visit(root, nil)
		end
		if not sameChildren(world:get(cts.Roots, cts.Roots), rootEntities) then
			world:set(cts.Roots, cts.Roots, rootEntities)
			dirty = true
		end

		for inst, entity in entities do
			if not seen[inst] and not inst:IsDescendantOf(game) then
				world:delete(entity)
				entities[inst] = nil
				dirty = true
			end
		end
	end

	-- Conflict entities live under their target's mirror entity; targets
	-- never change, so this settles after the first pass and self-heals.
	local function parentConflicts()
		for _, entity in (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {} do
			local targetEntity = entityFor(world:get(entity, cts.Target) :: Instance)
			if world:parent(entity) ~= targetEntity then
				world:add(entity, pair(ChildOf, targetEntity))
			end
		end
	end

	scheduler:addSystem({
		name = "mirror",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			debug.profilebegin("gitrbx_mirror")
			syncTree(treeRoots())
			parentConflicts()
			if dirty then
				dirty = false
				generation += 1
				world:set(cts.MirrorGeneration, cts.MirrorGeneration, generation)
			end
			debug.profileend()
		end,
	})
end
end function __DARKLUA_BUNDLE_MODULES.o():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.o if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.o=v end return v.c end end do local function __modImpl()--[[
	Builds immutable Explorer trees on the four view singleton entities.

	BASE/OURS/THEIRS come from complete virtual DOM manifests captured by Rust
	before the merge mutates anything. A logical node id is shared by matched
	instances across versions, while each manifest independently owns existence,
	parentage, order, name, and class. Concrete Instances are optional backing
	subjects; conflict snapshots replace them where exact branch data exists.

	RESULT follows the live mirror because staged decisions change the preview.
	Explorer hierarchy deliberately remains plain data rather than ECS ChildOf:
	one logical node can have three different parents or be absent in a version.
]]


local packages = __DARKLUA_BUNDLE_MODULES.d()
local types = __DARKLUA_BUNDLE_MODULES.m()







local function visibleChildren(inst: Instance): { Instance }
	local children = {}
	for _, child in inst:GetChildren() do
		if child.ClassName ~= "Highlight" and child.Name ~= "__GitRbxConflicts" then
			table.insert(children, child)
		end
	end
	return children
end

local function sourceKey(name: string, className: string): string
	return `{className}\0{name}`
end

return function(world: World__DARKLUA_TYPE_P, cts: Cts__DARKLUA_TYPE_Q, scheduler: Scheduler__DARKLUA_TYPE_R)
	local conflicts = (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {}
	local versionTrees = (world:get(cts.VersionTrees, cts.VersionTrees) :: VersionTrees__DARKLUA_TYPE_7?) or {}
	local subjects = (world:get(cts.VirtualSubjects, cts.VirtualSubjects) :: { [number]: Instance }?) or {}
	local subjectIds = (world:get(cts.VirtualSubjectIds, cts.VirtualSubjectIds) :: { [Instance]: number }?) or {}
	local baseParents: { [number]: number } = {}
	for _, record in versionTrees.base or {} do
		baseParents[record.id] = record.parent or 0
	end
	local versionChildren: { [string]: { [number]: { VersionTreeNode__DARKLUA_TYPE_6 } } } = {}
	for side, records in versionTrees do
		local children: { [number]: { VersionTreeNode__DARKLUA_TYPE_6 } } = {}
		for _, record in records do
			if record.parent then
				local bucket = children[record.parent] or {}
				children[record.parent] = bucket
				table.insert(bucket, record)
			end
		end
		versionChildren[side] = children
	end
	local sideViews = {
		base = cts.ExplorerBaseView,
		ours = cts.ExplorerOursView,
		theirs = cts.ExplorerTheirsView,
	}
	local sideBuilt: { [string]: boolean } = {}
	local resultGeneration = -1
	local resultResolutionSignature = ""
	local generation = 0

	local function finishSelectionIndex(projection: ExplorerProjection__DARKLUA_TYPE_5)
		local idIndex = projection.idIndex or {}
		local selectionIndex: { [number]: ExplorerNode__DARKLUA_TYPE_4 } = {}
		local resolved: { [number]: boolean } = {}
		for id, node in idIndex do
			selectionIndex[id] = node
			resolved[id] = true
		end

		local function nearest(id: number): ExplorerNode__DARKLUA_TYPE_4?
			if resolved[id] then
				return selectionIndex[id]
			end
			resolved[id] = true
			local parent = baseParents[id]
			if parent and parent ~= 0 then
				selectionIndex[id] = nearest(parent)
			end
			return selectionIndex[id]
		end
		for id in baseParents do
			nearest(id)
		end
		projection.selectionIndex = selectionIndex
	end

	local function bindSource(projection: ExplorerProjection__DARKLUA_TYPE_5, node: ExplorerNode__DARKLUA_TYPE_4, source: Instance, deep: boolean)
		node.source = source
		projection.index[source] = node
		if not deep then
			return
		end

		-- A delete-vs-edit clone contains the complete branch subtree. Match
		-- duplicate-name children by occurrence within their exact class/name
		-- bucket; the virtual manifest remains authoritative for topology.
		local buckets: { [string]: { Instance } } = {}
		for _, child in visibleChildren(source) do
			local key = sourceKey(child.Name, child.ClassName)
			local bucket = buckets[key] or {}
			buckets[key] = bucket
			table.insert(bucket, child)
		end
		local consumed: { [string]: number } = {}
		for _, childNode in node.children do
			local key = sourceKey(childNode.name, childNode.className or "")
			local index = (consumed[key] or 0) + 1
			consumed[key] = index
			local child = buckets[key] and buckets[key][index]
			if child then
				bindSource(projection, childNode, child, true)
			end
		end
	end

	local function buildVersion(tab: string): ExplorerProjection__DARKLUA_TYPE_5?
		local records = versionTrees[tab]
		if not records then
			return nil
		end
		local entityOf = (world:get(cts.EntityOf, cts.EntityOf) :: { [Instance]: any }?) or {}
		local projection: ExplorerProjection__DARKLUA_TYPE_5 = { roots = {}, index = {}, idIndex = {}, selectionIndex = {} }
		local idIndex = projection.idIndex :: { [number]: ExplorerNode__DARKLUA_TYPE_4 }

		for _, record: VersionTreeNode__DARKLUA_TYPE_6 in records do
			local subject = subjects[record.id]
			local node: ExplorerNode__DARKLUA_TYPE_4 = {
				id = record.id,
				source = subject,
				subject = subject,
				entity = if subject then entityOf[subject] else nil,
				name = record.name,
				className = record.className,
				children = {},
			}
			idIndex[record.id] = node
			if subject then
				projection.index[subject] = node
			end
		end

		for _, record: VersionTreeNode__DARKLUA_TYPE_6 in records do
			local node = idIndex[record.id]
			local parent = record.parent and idIndex[record.parent]
			node.parent = parent
			table.insert(if parent then parent.children else projection.roots, node)
		end

		local sideComponent = if tab == "ours" then cts.Ours elseif tab == "theirs" then cts.Theirs else nil
		if sideComponent then
			for _, conflict in conflicts do
				local target = world:get(conflict, cts.Target) :: Instance
				local node = projection.index[target]
				local info = world:get(conflict, sideComponent) :: SideInfo__DARKLUA_TYPE_3?
				if node and info and info.clone then
					bindSource(projection, node, info.clone, world:get(conflict, cts.Kind) == "DeleteVsEdit")
				end
			end
		end
		finishSelectionIndex(projection)

		return projection
	end

	local function buildResult(): ExplorerProjection__DARKLUA_TYPE_5
		local projection: ExplorerProjection__DARKLUA_TYPE_5 = { roots = {}, index = {}, idIndex = {}, selectionIndex = {} }
		-- Clone-backed previews retain the base target invisibly so decisions can
		-- be toggled. Do not expose that implementation target beside its clone.
		local hiddenRoots: { [Instance]: boolean } = {}
		for _, conflict in conflicts do
			local kind = world:get(conflict, cts.Kind)
			if kind == "DeleteVsEdit" then
				local staged = world:get(conflict, cts.Resolution) :: string?
				if staged then
					local ours = world:get(conflict, cts.Ours) :: SideInfo__DARKLUA_TYPE_3
					local theirs = world:get(conflict, cts.Theirs) :: SideInfo__DARKLUA_TYPE_3
					local deleteSide = if ours.deleted then "ours" elseif theirs.deleted then "theirs" else nil
					local keepSide = if deleteSide == "ours" then "theirs" else "ours"
					local keepInfo = if keepSide == "ours" then ours else theirs
					if
						staged == deleteSide
						or (staged == keepSide and (keepInfo.clone ~= nil or #(keepInfo.moveOuts or {}) > 0))
					then
						hiddenRoots[world:get(conflict, cts.Target) :: Instance] = true
					end
				end
			elseif kind == "PropertyBundle" then
				local staged = world:get(conflict, cts.Resolution) :: string?
				local info = if staged == "ours"
					then world:get(conflict, cts.Ours) :: SideInfo__DARKLUA_TYPE_3
					elseif staged == "theirs" then world:get(conflict, cts.Theirs) :: SideInfo__DARKLUA_TYPE_3
					else nil
				if info and info.clone then
					hiddenRoots[world:get(conflict, cts.Target) :: Instance] = true
				end
			end
		end

		local cloneIds: { [Instance]: number } = {}
		local function bindCloneIds(source: Instance, id: number, side: string)
			cloneIds[source] = id
			local childRecords = versionChildren[side] and versionChildren[side][id]
			if not childRecords then
				return
			end
			local buckets: { [string]: { Instance } } = {}
			for _, child in visibleChildren(source) do
				local key = sourceKey(child.Name, child.ClassName)
				local bucket = buckets[key] or {}
				buckets[key] = bucket
				table.insert(bucket, child)
			end
			local consumed: { [string]: number } = {}
			for _, record in childRecords do
				local key = sourceKey(record.name, record.className)
				local index = (consumed[key] or 0) + 1
				consumed[key] = index
				local child = buckets[key] and buckets[key][index]
				if child then
					bindCloneIds(child, record.id, side)
				end
			end
		end
		for _, conflict in conflicts do
			local id = subjectIds[world:get(conflict, cts.Target) :: Instance]
			if id then
				local ours = world:get(conflict, cts.Ours) :: SideInfo__DARKLUA_TYPE_3?
				local theirs = world:get(conflict, cts.Theirs) :: SideInfo__DARKLUA_TYPE_3?
				if ours and ours.clone then
					bindCloneIds(ours.clone, id, "ours")
				end
				if theirs and theirs.clone then
					bindCloneIds(theirs.clone, id, "theirs")
				end
			end
		end

		local idIndex = projection.idIndex :: { [number]: ExplorerNode__DARKLUA_TYPE_4 }
		local function visit(entity: any, parent: ExplorerNode__DARKLUA_TYPE_4?): ExplorerNode__DARKLUA_TYPE_4?
			local inst = world:get(entity, cts.Ref) :: Instance?
			if not inst or hiddenRoots[inst] then
				return nil
			end
			local id = subjectIds[inst] or cloneIds[inst]
			local node: ExplorerNode__DARKLUA_TYPE_4 = {
				id = id,
				source = inst,
				subject = inst,
				entity = entity,
				name = inst.Name,
				className = inst.ClassName,
				parent = parent,
				children = {},
			}
			projection.index[inst] = node
			if id then
				idIndex[id] = node
			end
			for _, child in (world:get(entity, cts.Children) :: { any }?) or {} do
				local childNode = visit(child, node)
				if childNode then
					table.insert(node.children, childNode)
				end
			end
			return node
		end
		for _, root in (world:get(cts.Roots, cts.Roots) :: { any }?) or {} do
			local node = visit(root, nil)
			if node then
				table.insert(projection.roots, node)
			end
		end
		finishSelectionIndex(projection)
		return projection
	end

	local function resolutionSignature(): string
		local states = table.create(#conflicts)
		for index, conflict in conflicts do
			states[index] = (world:get(conflict, cts.Resolution) :: string?) or "unresolved"
		end
		return table.concat(states, "|")
	end

	scheduler:addSystem({
		name = "explorer_projection",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			if world:has(cts.Finished, cts.Finished) then
				return
			end
			local mirrorGeneration = world:get(cts.MirrorGeneration, cts.MirrorGeneration) :: number?
			if not mirrorGeneration then
				return
			end

			local changed = false
			local signature = resolutionSignature()
			if resultGeneration ~= mirrorGeneration or resultResolutionSignature ~= signature then
				resultGeneration = mirrorGeneration
				resultResolutionSignature = signature
				world:set(cts.ExplorerResultView, cts.ExplorerProjection, buildResult())
				changed = true
			end
			local tab = world:get(cts.ViewTab, cts.ViewTab) :: string
			local view = sideViews[tab]
			if view and not sideBuilt[tab] then
				-- Version-1 artifacts had no complete manifests. Showing the full
				-- live tree is a truthful completeness fallback, though not an exact
				-- historical version; newly generated artifacts always use manifests.
				world:set(view, cts.ExplorerProjection, buildVersion(tab) or buildResult())
				sideBuilt[tab] = true
				changed = true
			end
			if changed then
				generation += 1
				world:set(cts.ExplorerGeneration, cts.ExplorerGeneration, generation)
			end
		end,
	})
end
end function __DARKLUA_BUNDLE_MODULES.p():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.p if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.p=v end return v.c end end do local function __modImpl()--[[
	VS Code-dark theme, lifted from the old web diff-viewer frontend
	(frontend/src/styles/index.css): #1e1e1e/#252526 surfaces, 1px #404040
	borders, monospace throughout, small radii, #094771 selection, #0e639c
	action blue, and the VS Code semantic palette for diff states.
]]


local wind = __DARKLUA_BUNDLE_MODULES.d()
.wind
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
theme.reparented = Color3.fromRGB(78, 193, 255) -- #4ec1ff
theme.pivoted = Color3.fromRGB(206, 145, 120) -- #ce9178
theme.warning = Color3.fromRGB(204, 167, 0) -- #cca700
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
theme.fontFace = Font.fromEnum(Enum.Font.RobotoMono)
theme.fontFaceBold = Font.new(theme.fontFace.Family, Enum.FontWeight.Bold)
theme.textSize = 14
theme.textSizeSmall = 12
theme.headerSize = 12

-- Each render surface needs its own sheet link: the wind utility classes
-- (flex-*, gap-*, p-*, corner-*, auto-size-*, bg-none, ...) plus the
-- resolver's VS Code-dark classes below.
function theme.link()
	debug.profilebegin("gitrbx_sheet")
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

	-- Row backgrounds: selected > on-path > stripe over an invisible `.row`
	-- base, so a row just carries whichever state tags apply.
	StyleRule(".row", { BackgroundColor3 = theme.bg, BackgroundTransparency = 1 })
	StyleRule(".stripe", { BackgroundColor3 = theme.bg, BackgroundTransparency = 0.5 }, nil, 1)
	StyleRule(".on-path", { BackgroundColor3 = theme.selection, BackgroundTransparency = 0.85 }, nil, 2)
	StyleRule(".selected", { BackgroundColor3 = theme.selection, BackgroundTransparency = 0 }, nil, 3)
end

return theme
end function __DARKLUA_BUNDLE_MODULES.q():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.q if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.q=v end return v.c end end do local function __modImpl()--[[
	Outline-only highlight on every conflict target (or the swapped-in clone
	a staged delete-vs-edit edit side renders), tinted by staged resolution.
	Resolution vs Highlighted drives the frame diff — Highlight instances are
	only touched when a conflict's staged state actually changes, mirroring
	the Rendered pattern in renderPreviews.
]]

local packages = __DARKLUA_BUNDLE_MODULES.d()

local theme = __DARKLUA_BUNDLE_MODULES.q()

local conflicts = __DARKLUA_BUNDLE_MODULES.j()
local types = __DARKLUA_BUNDLE_MODULES.m()

local jecs = packages.jecs

return function(world: World__DARKLUA_TYPE_P, cts: Cts__DARKLUA_TYPE_Q, scheduler: Scheduler__DARKLUA_TYPE_R)
	local conflictList = (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {}

	-- System-private Highlight instances, keyed by conflict entity.
	local liveHighlights: { [any]: Highlight } = {}

	local function applyHighlight(entity: any, resolution: string?)
		local target = world:get(entity, cts.Target) :: Instance

		local highlight = liveHighlights[entity]
		if not highlight then
			highlight = Instance.new("Highlight")
			highlight.Name = `GitRbxHighlight_{world:get(entity, jecs.Name)}`
			highlight.Parent = target
			liveHighlights[entity] = highlight
		end

		-- Adorn whatever the preview is actually rendering: normally the
		-- target, but a staged keep side of a delete-vs-edit renders its
		-- swapped-in clone instead.
		highlight.Adornee = conflicts.renderedInstance(entity)

		local color = theme.statusColor(resolution)
		highlight.OutlineColor = color
		highlight.FillColor = color
		highlight.FillTransparency = 1 -- outline only
	end

	local function removeHighlight(entity: any)
		local highlight = liveHighlights[entity]
		if highlight then
			highlight:Destroy()
			liveHighlights[entity] = nil
		end
	end

	scheduler:addSystem({
		name = "update_highlights",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			debug.profilebegin("gitrbx_highlights")
			local finished = world:has(cts.Finished, cts.Finished)
			for _, entity in conflictList do
				local target = world:get(entity, cts.Target) :: Instance
				local spatial = target:IsDescendantOf(workspace) and target:IsA("PVInstance")

				local wanted: string? = nil
				if spatial and not finished then
					wanted = (world:get(entity, cts.Resolution) :: string?) or "unresolved"
				end

				if world:get(entity, cts.Highlighted) ~= wanted then
					world:set(entity, cts.Highlighted, wanted)
					if wanted == nil then
						removeHighlight(entity)
					else
						applyHighlight(entity, if wanted == "unresolved" then nil else wanted)
					end
				end
			end
			debug.profileend()
		end,
	})
end
end function __DARKLUA_BUNDLE_MODULES.r():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.r if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.r=v end return v.c end end do local function __modImpl()-- (center CFrame, size) for a Model, BasePart, or a container whose
-- descendants include BaseParts. Container bounds are world-axis-aligned and
-- include all eight corners of rotated parts, so Folder conflict targets can
-- be framed without widening the selection to their parent model.
-- An optional pivot folds the box back into the pivot's frame so framing is
-- measured relative to where the model is "held" rather than its raw bounds.
-- Ported from townies viewports/Utils/getTrueBoundingBox.

local function getBoundingBox(root: Instance, pivot: PVInstance?): (CFrame, Vector3)
	local cframe, size

	if root:IsA("Model") then
		cframe, size = root:GetBoundingBox()
	elseif root:IsA("BasePart") then
		cframe, size = root.CFrame, root.Size
	else
		local minimum = Vector3.new(math.huge, math.huge, math.huge)
		local maximum = Vector3.new(-math.huge, -math.huge, -math.huge)
		local partCount = 0
		for _, descendant in root:GetDescendants() do
			if descendant:IsA("BasePart") then
				partCount += 1
				local halfSize = descendant.Size / 2
				for x = -1, 1, 2 do
					for y = -1, 1, 2 do
						for z = -1, 1, 2 do
							local corner = descendant.CFrame:PointToWorldSpace(
								Vector3.new(x * halfSize.X, y * halfSize.Y, z * halfSize.Z)
							)
							minimum = Vector3.new(
								math.min(minimum.X, corner.X),
								math.min(minimum.Y, corner.Y),
								math.min(minimum.Z, corner.Z)
							)
							maximum = Vector3.new(
								math.max(maximum.X, corner.X),
								math.max(maximum.Y, corner.Y),
								math.max(maximum.Z, corner.Z)
							)
						end
					end
				end
			end
		end
		assert(partCount > 0, `{root:GetFullName()} has no spatial descendants`)
		cframe = CFrame.new((minimum + maximum) / 2)
		size = maximum - minimum
	end

	if pivot and pivot ~= root then
		local pivotCF = pivot:GetPivot()
		local relativeCF = pivotCF:ToObjectSpace(cframe)
		cframe = cframe * CFrame.new(-relativeCF.Position) * relativeCF.Rotation:Inverse()
	end

	return cframe, size
end

return getBoundingBox
end function __DARKLUA_BUNDLE_MODULES.s():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.s if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.s=v end return v.c end end do local function __modImpl()-- Distance a camera needs in order to fully frame `targetSize` at the given
-- `fov` (degrees) and viewport aspect ratio. Considers horizontal, vertical and
-- depth extents, then adds 10% padding.
-- Ported from townies viewports/Utils/getIdealCameraDistanceToContainModel.

local function getIdealCameraDistance(fov: number, targetSize: Vector3, aspectRatio: number?): number
	aspectRatio = aspectRatio or 16 / 9

	local halfFOV = math.rad(fov) / 2
	local x, y, z = targetSize.X, targetSize.Y, targetSize.Z

	local horizontalDist = (x / 2) / math.tan(halfFOV)
	local verticalDist = (y / 2) / math.tan(halfFOV / aspectRatio)
	local depthDist = z / 2

	local maxDist = math.max(horizontalDist, verticalDist) + depthDist
	return maxDist * 1.1 -- 10% padding for safety
end

return getIdealCameraDistance
end function __DARKLUA_BUNDLE_MODULES.t():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.t if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.t=v end return v.c end end do local function __modImpl()-- Camera direction presets: the unit-ish vector pointing FROM the model TOWARD
-- the camera. Ported from townies viewports/Utils/getIdealCameraViewCFrame.

return {
	default = vector.create(1, -0.5, 1),
	front = vector.create(0, 0, 1),
	side = vector.create(1, 0, 0),
	top = vector.create(0, -1, 0),
	isometric = vector.create(1, -1, 1),
	heroic = vector.create(0.3, 0.5, 1), -- low angle, flatters characters
}
end function __DARKLUA_BUNDLE_MODULES.u():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.u if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.u=v end return v.c end end do local function __modImpl()
local getIdealCameraDistance = __DARKLUA_BUNDLE_MODULES.t()
local presets = __DARKLUA_BUNDLE_MODULES.u()

-- Camera CFrame that frames a model whose bounds are (modelCF, modelSize).
-- Ported from townies viewports/Utils/getIdealCameraViewCFrame (options type
-- added for this strict-mode vendor).










local function getIdealCameraCFrame(modelCF: CFrame, modelSize: Vector3, optionsIn: Options__DARKLUA_TYPE_8?): CFrame
	local options: Options__DARKLUA_TYPE_8 = optionsIn or {}

	local fov = options.fov or 70
	local scale = options.scale or 1
	local preset = options.preset or "default"
	local direction = options.direction or presets[preset] or presets.default
	local aspectRatio = options.aspectRatio or 1
	local offset = options.offset or Vector3.zero

	local distance = getIdealCameraDistance(fov, modelSize, aspectRatio)
	local scaledDistance = distance * scale
	local position = modelCF.Position + offset - direction.Unit * scaledDistance

	if options.rotation then
		local angle = math.rad(options.rotation)
		local rotatedDir = CFrame.Angles(0, angle, 0) * direction
		position = modelCF.Position + offset - rotatedDir.Unit * scaledDistance
	end

	return CFrame.lookAt(position, modelCF.Position + offset)
end

return getIdealCameraCFrame
end function __DARKLUA_BUNDLE_MODULES.v():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.v if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.v=v end return v.c end end do local function __modImpl()--[[
	Camera focus as request entities: the View button spawns an entity with a
	FocusRequest payload (the conflict entity to frame); this system consumes
	every pending request and tags it Destroy for the cleanup sweep.

	Framing is a fitted distance from the target's bounding box and the live
	camera FOV/aspect (vendored miu-viewport math). Focus is the edit
	camera's orbit pivot: it must be written together with the CFrame, or
	Studio keeps steering around the stale pivot and the view drifts off
	target.
]]


local packages = __DARKLUA_BUNDLE_MODULES.d()

local conflicts = __DARKLUA_BUNDLE_MODULES.j()
local types = __DARKLUA_BUNDLE_MODULES.m()

local getBoundingBox = __DARKLUA_BUNDLE_MODULES.s()
local getIdealCameraCFrame = __DARKLUA_BUNDLE_MODULES.v()

return function(world: World__DARKLUA_TYPE_P, cts: Cts__DARKLUA_TYPE_Q, scheduler: Scheduler__DARKLUA_TYPE_R)
	local function focusOn(conflictEntity: any)
		local camera = workspace.CurrentCamera
		-- Frame what the preview is actually rendering — the staged clone
		-- when a delete-vs-edit keep side is staged, the target otherwise.
		local target = conflicts.renderedInstance(conflictEntity)
		local cframe, size = getBoundingBox(target)
		camera.CFrame = getIdealCameraCFrame(cframe, size, {
			fov = camera.FieldOfView,
			aspectRatio = camera.ViewportSize.X / camera.ViewportSize.Y,
		})
		camera.Focus = cframe
	end

	local focusRequests = world:query(cts.FocusRequest):cached()

	scheduler:addSystem({
		name = "focus_camera",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			for requestEntity, conflictEntity in focusRequests do
				focusOn(conflictEntity)
				world:add(requestEntity, cts.Destroy)
			end
		end,
	})
end
end function __DARKLUA_BUNDLE_MODULES.w():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.w if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.w=v end return v.c end end do local function __modImpl()--[[
	Dock widget creation. rodeo runs us at plugin identity but hands us no
	`plugin` global — the rodeo plugin itself is a Plugin instance under
	PluginDebugService, and CreateDockWidgetPluginGuiAsync works from there.
]]


local widgets = {}








function widgets.create(id: string, title: string, dockState: Enum.InitialDockState, size: Size__DARKLUA_TYPE_9?): DockWidgetPluginGui
	local rodeoPlugin: Plugin?
	for _, child in game:GetService("PluginDebugService"):GetChildren() do
		if child:IsA("Plugin") and child.Name:match("rodeo") then
			rodeoPlugin = child
			break
		end
	end

	local widget = (rodeoPlugin :: Plugin):CreateDockWidgetPluginGuiAsync(
		id,
		DockWidgetPluginGuiInfo.new(
			dockState,
			true,
			true,
			if size and size.width then size.width else 300,
			if size and size.height then size.height else 600,
			if size and size.minWidth then size.minWidth else 240,
			if size and size.minHeight then size.minHeight else 200
		)
	)
	widget.Title = title
	return widget
end

return widgets
end function __DARKLUA_BUNDLE_MODULES.x():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.x if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.x=v end return v.c end end do local function __modImpl()--[[
	Studio's own explorer icons (spritesheet slices), cached per class — the
	same iconography as the native Explorer, icon packs respected.
]]


local StudioService = game:GetService("StudioService")



local cache: { [string]: ClassIcon__DARKLUA_TYPE_aa } = {}

local function classIcon(className: string): ClassIcon__DARKLUA_TYPE_aa
	local icon = cache[className]
	if not icon then
		icon = StudioService:GetClassIcon(className) :: ClassIcon__DARKLUA_TYPE_aa
		cache[className] = icon
	end
	return icon
end

return classIcon
end function __DARKLUA_BUNDLE_MODULES.y():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.y if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.y=v end return v.c end end do local function __modImpl()-- @generated by rbx-iconify

local INDEX = {
	ui = {
		tile = 16,
		columns = 1,
		icons = {
			["bxs:down-arrow"] = { 1, 0 },
			["bxs:right-arrow"] = { 1, 1 },
			["lucide:eye"] = { 1, 2 },
			["lucide:minus"] = { 1, 3 },
			["lucide:move-right"] = { 1, 4 },
			["lucide:pencil"] = { 1, 5 },
			["lucide:plus"] = { 1, 6 },
			["lucide:rotate-3d"] = { 1, 7 },
		},
	},
}

local function iconify(name: string, target: string)
	local data = INDEX[target]
	local entry = data.icons[name]
	local col = entry[2] % data.columns
	local row = math.floor(entry[2] / data.columns)
	return {
		sheet = entry[1],
		offset = Vector2.new(col * data.tile, row * data.tile),
		size = Vector2.new(data.tile, data.tile),
	}
end

return iconify
end function __DARKLUA_BUNDLE_MODULES.z():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.z if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.z=v end return v.c end end do local function __modImpl()-- This file is automatically @generated by Asphalt.
-- It is not intended for manual editing.

local images = {
	iconify = {
		ui = {
			["1"] = "rbxasset://.asphalt-git-rbx/32fbadd7d29730532ab76c18ddeb73a9c24bd3beb2de6e8ec7ac0714f5ab5137.png",
		},
	},
}

return images end function __DARKLUA_BUNDLE_MODULES.A():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.A if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.A=v end return v.c end end do local function __modImpl()
local Element = __DARKLUA_BUNDLE_MODULES.d()
.miu.Element

local iconify = __DARKLUA_BUNDLE_MODULES.z()
local images = __DARKLUA_BUNDLE_MODULES.A()

local function Icon(id: string, name: string, color: Color3, size: UDim2?)
	local data = iconify(name, "ui")
	Element("ImageLabel", {
		id = id,
		Image = images.iconify.ui[tostring(data.sheet)],
		ImageRectOffset = data.offset,
		ImageRectSize = data.size,
		ImageColor3 = color,
		Size = size or UDim2.fromOffset(data.size.X, data.size.Y),
		AnchorPoint = Vector2.new(0.5, 0.5),
		Position = UDim2.fromScale(0.5, 0.5),
		BackgroundTransparency = 1,
	})
end

return Icon
end function __DARKLUA_BUNDLE_MODULES.B():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.B if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.B=v end return v.c end end do local function __modImpl()--[[
	Presentational tree row, after game-prototype's miu-tree/TreeRow: same
	props contract (depth, label, hasChildren, expanded, selected, onPath,
	onActivated, onToggle) adapted to the resolver's stack — plain miu events
	instead of the miu-flow Activated helper — plus resolver extras: a class
	icon and a phantom dim.

	Background states are wind tags: selected > on-path > stripe, precedence
	via rule priority in theme.rules().
]]

local packages = __DARKLUA_BUNDLE_MODULES.d()
local Element = packages.miu.Element
local tags = packages.wind.tags

local classIcon = __DARKLUA_BUNDLE_MODULES.y()
local Icon = __DARKLUA_BUNDLE_MODULES.B()
local theme = __DARKLUA_BUNDLE_MODULES.q()


















local INDENT = 12

local function TreeRow(props: TreeRowProps__DARKLUA_TYPE_ab)
	Element("Frame", {
		id = props.id,
		LayoutOrder = props.layoutOrder,
		Size = UDim2.new(1, 0, 0, 22),
		tags = tags("row", props.stripe and "stripe", props.onPath and "on-path", props.selected and "selected"),
	}, function()
		Element("UIPadding", {
			id = "pad",
			PaddingLeft = UDim.new(0, 4 + props.depth * INDENT),
		})
		Element("UIListLayout", {
			id = "layout",
			FillDirection = Enum.FillDirection.Horizontal,
			SortOrder = Enum.SortOrder.LayoutOrder,
			VerticalAlignment = Enum.VerticalAlignment.Center,
		})

		Element("TextButton", {
			id = "expand",
			LayoutOrder = 1,
			Text = "",
			Size = UDim2.new(0, 16, 1, 0),
			AutoButtonColor = false,
			tags = { "bg-none" },
			events = {
				Activated = function()
					if props.onToggle then
						props.onToggle()
					end
				end,
			},
		}, function()
			if props.hasChildren then
				Icon(
					"disclosure",
					if props.expanded then "bxs:down-arrow" else "bxs:right-arrow",
					theme.textMuted,
					UDim2.fromOffset(8, 8)
				)
			end
		end)

		local iconWidth = 0
		if props.className then
			iconWidth = 16
			local icon = classIcon(props.className :: string)
			Element("ImageLabel", {
				id = "icon",
				LayoutOrder = 2,
				Image = icon.Image,
				ImageRectOffset = icon.ImageRectOffset,
				ImageRectSize = icon.ImageRectSize,
				ImageTransparency = if props.phantom then 0.5 else 0,
				Size = UDim2.new(0, 16, 0, 16),
				BackgroundTransparency = 1,
			})
		end

		Element("TextButton", {
			id = "label",
			LayoutOrder = 3,
			RichText = true,
			Text = ` {props.label}`,
			Size = UDim2.new(1, -16 - iconWidth, 1, 0),
			tags = { "text" },
			TextColor3 = props.color or theme.text,
			TextTruncate = Enum.TextTruncate.AtEnd,
			events = {
				Activated = function()
					if props.onActivated then
						props.onActivated()
					end
				end,
			},
		})
	end)
end

return TreeRow
end function __DARKLUA_BUNDLE_MODULES.C():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.C if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.C=v end return v.c end end do local function __modImpl()--[[
	Plain explorer over the merge's four states, tabbed BASE | OURS | THEIRS
	| RESULT. No diff presentation here yet (badges, tints, phantoms) — this
	is a normal explorer per state; the diff layers come back iteratively.

	All four tabs walk ECS-owned Explorer projection tables. Nodes share
	canonical subjects but carry view-specific sources and parents, so reparented,
	deleted, cloned, or virtualized content needs no UI-side tree grafting.

	Conflict ancestor chains start expanded so each contested root is visible,
	but subtree contents start collapsed. Search temporarily forces every branch
	open so hidden descendants remain discoverable, then shows each name/class
	match with its visual ancestor chain.
]]


local packages = __DARKLUA_BUNDLE_MODULES.d()
local Element = packages.miu.Element
local VirtualList = packages.virtualList.VirtualList
local Dimensions = packages.virtualList.Dimensions

local TreeRow = __DARKLUA_BUNDLE_MODULES.C()

local format = __DARKLUA_BUNDLE_MODULES.g()
local theme = __DARKLUA_BUNDLE_MODULES.q()

local cts = __DARKLUA_BUNDLE_MODULES.i()
local world = __DARKLUA_BUNDLE_MODULES.h()


















local VIEW_ENTITIES = {
	base = cts.ExplorerBaseView,
	ours = cts.ExplorerOursView,
	theirs = cts.ExplorerTheirsView,
	result = cts.ExplorerResultView,
}

local expanded: { [any]: boolean } = {}
local expansionGen = 0
local seededViews: { [any]: boolean } = {}
local lastRevealedInstance: Instance? = nil
local lastRevealedId: number? = nil
local lastRevealedView: any? = nil
local lastRevealedGeneration = -1
local lastRevealRequest = -1
local pendingCenterNode: ExplorerNode__DARKLUA_TYPE_ac? = nil
local pendingCenterView: any? = nil
local searchText = ""
local searchGen = 0

local function setExpanded(key: any, isExpanded: boolean)
	expanded[key] = isExpanded
	expansionGen += 1
end

local function expansionKey(node: ExplorerNode__DARKLUA_TYPE_ac): any
	return if node.id then `virtual:{node.id}` else node.subject or node.source or node
end

local function selectedNode(projection: ExplorerProjection__DARKLUA_TYPE_ad): ExplorerNode__DARKLUA_TYPE_ac?
	local id = world:get(cts.SelectedExplorerId, cts.SelectedExplorerId) :: number?
	local node = id and projection.selectionIndex and projection.selectionIndex[id]
	if node then
		return node
	end
	local inst = world:get(cts.SelectedInstance, cts.SelectedInstance) :: Instance?
	return inst and projection.index[inst]
end

local function seedExpansion(view: any, projection: ExplorerProjection__DARKLUA_TYPE_ad)
	if seededViews[view] then
		return
	end
	seededViews[view] = true
	expansionGen += 1
	local subjectIds = (world:get(cts.VirtualSubjectIds, cts.VirtualSubjectIds) :: { [Instance]: number }?) or {}
	for _, target in world:query(cts.Target) do
		local inst = target :: Instance
		local id = subjectIds[inst]
		local node = (id and projection.selectionIndex and projection.selectionIndex[id]) or projection.index[inst]
		node = node and node.parent
		while node do
			expanded[expansionKey(node)] = true
			node = node.parent
		end
	end
end

-- Expand the selected logical node's ancestor chain when the selection changes
-- (e.g. clicked from the conflict list) — but only then, so a manual
-- collapse afterwards sticks.
local function revealSelected(view: any, projection: ExplorerProjection__DARKLUA_TYPE_ad, explorerGeneration: number)
	local inst = world:get(cts.SelectedInstance, cts.SelectedInstance) :: Instance?
	local id = world:get(cts.SelectedExplorerId, cts.SelectedExplorerId) :: number?
	local revealRequest = (world:get(cts.ExplorerRevealGeneration, cts.ExplorerRevealGeneration) :: number?) or 0
	local viewChanged = view ~= lastRevealedView
	local explicitReveal = revealRequest ~= lastRevealRequest
	if
		(inst == nil and id == nil)
		or (
			inst == lastRevealedInstance
			and id == lastRevealedId
			and view == lastRevealedView
			and explorerGeneration == lastRevealedGeneration
			and revealRequest == lastRevealRequest
		)
	then
		return
	end
	lastRevealedInstance = inst
	lastRevealedId = id
	lastRevealedView = view
	lastRevealedGeneration = explorerGeneration
	lastRevealRequest = revealRequest
	expansionGen += 1
	local node = selectedNode(projection)
	if not node then
		return
	end
	if explicitReveal and searchText ~= "" then
		searchText = ""
		searchGen += 1
	end
	-- Center explicit conflict-list selections and carry that selection across
	-- version tabs. Clicking an already-visible Explorer row should not make
	-- the list jump underneath the pointer.
	if explicitReveal or viewChanged then
		pendingCenterNode = node
		pendingCenterView = view
	end
	node = node.parent
	while node do
		expanded[expansionKey(node)] = true
		node = node.parent
	end
end

local function explorerRow(node: ExplorerNode__DARKLUA_TYPE_ac, depth: number, hasChildren: boolean, isExpanded: boolean): Row__DARKLUA_TYPE_ae
	local classSuffix = if node.className then `  <font color="{theme.textDimHex}">{node.className}</font>` else ""
	return {
		node = node,
		source = node.source,
		subject = node.subject,
		expansionKey = expansionKey(node),
		label = `{format.escapeRich(node.name)}{classSuffix}`,
		searchText = string.lower(`{node.name} {node.className or ""}`),
		className = node.className,
		phantom = node.phantom,
		depth = depth,
		hasChildren = hasChildren,
		isExpanded = isExpanded,
	}
end

local function projectionRows(projection: ExplorerProjection__DARKLUA_TYPE_ad, forceExpanded: boolean): { Row__DARKLUA_TYPE_ae }
	local rows: { Row__DARKLUA_TYPE_ae } = {}
	local function visit(node: ExplorerNode__DARKLUA_TYPE_ac, depth: number)
		local isExpanded = forceExpanded or expanded[expansionKey(node)] == true
		table.insert(rows, explorerRow(node, depth, #node.children > 0, isExpanded))
		if isExpanded then
			for _, child in node.children do
				visit(child, depth + 1)
			end
		end
	end
	for _, root in projection.roots do
		visit(root, 0)
	end
	return rows
end

local TABS = { "base", "ours", "theirs", "result" }

local function tabColor(tab: string): Color3
	if tab == "ours" then
		return theme.ours
	elseif tab == "theirs" then
		return theme.theirs
	elseif tab == "base" then
		return theme.textMuted
	end
	return theme.text
end

local function TabBar()
	local viewTab = world:get(cts.ViewTab, cts.ViewTab)
	Element("Frame", {
		id = "tabs",
		Size = UDim2.new(1, 0, 0, 26),
		tags = { "bg-none", "flex-row", "gap-xs" },
	}, function()
		Element("UIPadding", {
			id = "pad",
			PaddingLeft = UDim.new(0, 4),
			PaddingRight = UDim.new(0, 4),
			PaddingTop = UDim.new(0, 3),
			PaddingBottom = UDim.new(0, 3),
		})
		for _, tab in TABS do
			local isActive = viewTab == tab
			Element("TextButton", {
				id = tab,
				Text = tab:upper(),
				Size = UDim2.new(0.25, -8, 1, 0),
				tags = { "corner-sm", "vs-border", "button" },
				BackgroundColor3 = if isActive then theme.selection else theme.surface,
				TextColor3 = if isActive then theme.textBright else tabColor(tab),
				TextSize = 11,
				events = {
					Activated = function()
						world:set(cts.ViewTab, cts.ViewTab, tab)
					end,
				},
			})
		end
	end)
end

-- Ancestors of the selected occurrence light up as on-path. Following the
-- projected parent (rather than Instance.Parent) gives reparent views and virtual
-- nodes their own topology.
local function ancestorSet(projection: ExplorerProjection__DARKLUA_TYPE_ad): { [ExplorerNode__DARKLUA_TYPE_ac]: boolean }
	local set: { [any]: boolean } = {}
	local node = selectedNode(projection)
	node = node and node.parent
	while node do
		set[node] = true
		node = node.parent
	end
	return set
end

local ROW_HEIGHT = 22
local HEADER_HEIGHT = 24
local TABS_HEIGHT = 26
local SEARCH_HEIGHT = 30

local rowsCacheKey: string? = nil
local rowsCache: { Row__DARKLUA_TYPE_ae } = {}

-- Rows render through VirtualList: only the visible index range builds
-- TreeRows, so even a large search result stays cheap.
local function renderRows(rows: { Row__DARKLUA_TYPE_ae }, searching: boolean, projection: ExplorerProjection__DARKLUA_TYPE_ad, view: any)
	local onPath = ancestorSet(projection)
	local selected = selectedNode(projection)
	VirtualList.push({
		id = "rows",
		data = rows,
		dimensions = Dimensions.consistentSize(ROW_HEIGHT),
		direction = "y",
		Size = UDim2.new(1, 0, 1, -HEADER_HEIGHT - TABS_HEIGHT - SEARCH_HEIGHT),
		tags = { "bg-none" },
	}, function(row: Row__DARKLUA_TYPE_ae, index: number)
		TreeRow({
			id = "row",
			depth = row.depth,
			label = row.label,
			className = row.className,
			phantom = row.phantom,
			hasChildren = row.hasChildren,
			expanded = row.isExpanded,
			selected = row.node == selected,
			onPath = onPath[row.node] == true,
			stripe = index % 2 == 0,
			onToggle = if row.hasChildren and not searching
				then function()
					setExpanded(row.expansionKey, not row.isExpanded)
				end
				else nil,
			onActivated = if row.source
					or row.subject
					or row.node.id
				then function()
					local instance = row.source or row.subject
					if instance then
						world:set(cts.SelectedInstance, cts.SelectedInstance, instance)
					else
						world:remove(cts.SelectedInstance, cts.SelectedInstance)
					end
					if row.node.id then
						world:set(cts.SelectedExplorerId, cts.SelectedExplorerId, row.node.id)
					else
						world:remove(cts.SelectedExplorerId, cts.SelectedExplorerId)
					end
				end
				else nil,
		})
	end)

	-- VirtualList keeps its overlay scope open until pop(). Its retained parent
	-- owns the real ScrollingFrame, so an explicit Select can center the row
	-- after ancestor expansion has rebuilt this frame's flattened row array.
	if pendingCenterView == view and pendingCenterNode then
		local centerIndex: number? = nil
		for index, row in rows do
			if row.node == pendingCenterNode then
				centerIndex = index
				break
			end
		end
		local overlayNode = packages.miu.peek()
		local outerNode = overlayNode.parent
		local scrollNode = outerNode and outerNode.children.scroll
		local scroll = if scrollNode then scrollNode.instance :: ScrollingFrame else nil
		if centerIndex and scroll and scroll.AbsoluteWindowSize.Y > 0 then
			local viewportHeight = scroll.AbsoluteWindowSize.Y
			local canvasHeight = #rows * ROW_HEIGHT
			local target = (centerIndex - 0.5) * ROW_HEIGHT - viewportHeight / 2
			target = math.clamp(target, 0, math.max(0, canvasHeight - viewportHeight))
			scroll.CanvasPosition = Vector2.new(scroll.CanvasPosition.X, target)
			pendingCenterNode = nil
			pendingCenterView = nil
		end
	end
	VirtualList.pop()
end

local function normalizedSearch(): string
	return string.lower((searchText:match("^%s*(.-)%s*$") :: string?) or "")
end

local function rowMatches(row: Row__DARKLUA_TYPE_ae, query: string): boolean
	for token in query:gmatch("%S+") do
		if not row.searchText:find(token, 1, true) then
			return false
		end
	end
	return true
end

-- Rows are pre-order, so each match's nearest preceding shallower rows are
-- precisely its projected visual ancestors even when Instance.Parent differs.
local function filterRows(rows: { Row__DARKLUA_TYPE_ae }, query: string): { Row__DARKLUA_TYPE_ae }
	if query == "" then
		return rows
	end

	local keep: { [number]: boolean } = {}
	for index, row in rows do
		if not rowMatches(row, query) then
			continue
		end
		keep[index] = true
		local ancestorDepth = row.depth
		for candidate = index - 1, 1, -1 do
			local candidateRow = rows[candidate]
			if candidateRow.depth < ancestorDepth then
				keep[candidate] = true
				ancestorDepth = candidateRow.depth
				if ancestorDepth == 0 then
					break
				end
			end
		end
	end

	local filtered: { Row__DARKLUA_TYPE_ae } = {}
	for index, row in rows do
		if keep[index] then
			table.insert(filtered, row)
		end
	end
	return filtered
end

local function SearchBar()
	Element("Frame", {
		id = "search-wrap",
		Size = UDim2.new(1, 0, 0, SEARCH_HEIGHT),
		tags = { "bg-none" },
	}, function()
		Element("UIPadding", {
			id = "pad",
			PaddingLeft = UDim.new(0, 4),
			PaddingRight = UDim.new(0, 4),
			PaddingTop = UDim.new(0, 3),
			PaddingBottom = UDim.new(0, 3),
		})

		local textBox: TextBox
		local node = Element("TextBox", {
			id = "search",
			Text = searchText,
			PlaceholderText = "Search name or class…",
			ClearTextOnFocus = false,
			Size = UDim2.fromScale(1, 1),
			tags = { "text", "surface-high", "vs-border", "corner-sm" },
			TextColor3 = theme.text,
			PlaceholderColor3 = theme.textMuted,
			TextSize = theme.textSizeSmall,
			events = {
				Changed = function(property: string)
					if property ~= "Text" or textBox.Text == searchText then
						return
					end
					searchText = textBox.Text
					searchGen += 1
				end,
			},
		})
		textBox = node.instance :: TextBox
	end)
end

local function ExplorerPanel()
	Element("Frame", {
		id = "explorer",
		Size = UDim2.fromScale(1, 1),
		tags = { "bg-none", "flex-col" },
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = "MERGE EXPLORER",
			Size = UDim2.new(1, 0, 0, HEADER_HEIGHT),
			tags = { "header-band" },
		})

		if world:has(cts.Finished, cts.Finished) then
			return
		end

		TabBar()
		SearchBar()

		local viewTab = world:get(cts.ViewTab, cts.ViewTab) :: string
		local view = VIEW_ENTITIES[viewTab]
		local explorerGen = (world:get(cts.ExplorerGeneration, cts.ExplorerGeneration) :: number?) or 0
		local projection = view and world:get(view, cts.ExplorerProjection) :: ExplorerProjection__DARKLUA_TYPE_ad?
		-- Systems begin running as they are registered. The first UI frame can
		-- therefore precede the mirror/projection's initial Heartbeat; an empty
		-- panel is the correct intermediate state and avoids querying a nil view.
		if not view or not projection or explorerGen == 0 then
			return
		end
		seedExpansion(view, projection)
		revealSelected(view, projection, explorerGen)

		-- Rows are derived data: rebuilt only when the projected tree,
		-- expansion, or tab changes. Selection/on-path stay out of the key —
		-- renderRows applies them per frame over the cached array.
		local query = normalizedSearch()
		local searching = query ~= ""
		local key = `{viewTab}:{expansionGen}:{explorerGen}:{searchGen}`
		if key ~= rowsCacheKey then
			rowsCacheKey = key
			debug.profilebegin("gitrbx_explorer_rows")
			local rows = projectionRows(projection, searching)
			rowsCache = filterRows(rows, query)
			debug.profileend()
		end
		renderRows(rowsCache, searching, projection, view)
	end)
end

return ExplorerPanel
end function __DARKLUA_BUNDLE_MODULES.D():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.D if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.D=v end return v.c end end do local function __modImpl()-- Explorer dock widget, rendered immediate-mode every heartbeat.


local packages = __DARKLUA_BUNDLE_MODULES.d()
local Element = packages.miu.Element
local Portal = packages.miu.Portal

local theme = __DARKLUA_BUNDLE_MODULES.q()

local types = __DARKLUA_BUNDLE_MODULES.m()
local widgets = __DARKLUA_BUNDLE_MODULES.x()

local ExplorerPanel = __DARKLUA_BUNDLE_MODULES.D()

return function(_world: World__DARKLUA_TYPE_P, _cts: Cts__DARKLUA_TYPE_Q, scheduler: Scheduler__DARKLUA_TYPE_R)
	local widget = widgets.create("GitRbxMergeExplorer", "git-rbx — explorer", Enum.InitialDockState.Left, {
		width = 360,
		height = 600,
		minWidth = 260,
		minHeight = 240,
	})

	scheduler:addSystem({
		name = "ui_explorer",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			debug.profilebegin("gitrbx_ui_explorer")
			Portal(widget, "git-rbx-explorer", function()
				theme.link()
				Element("Frame", {
					id = "explorer-widget",
					Size = UDim2.fromScale(1, 1),
					tags = { "surface", "vs-border" },
				}, function()
					ExplorerPanel()
				end)
			end)
			debug.profileend()
		end,
	})
end
end function __DARKLUA_BUNDLE_MODULES.E():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.E if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.E=v end return v.c end end do local function __modImpl()
local Element = __DARKLUA_BUNDLE_MODULES.d()
.miu.Element

local format = __DARKLUA_BUNDLE_MODULES.g()
local theme = __DARKLUA_BUNDLE_MODULES.q()

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
end function __DARKLUA_BUNDLE_MODULES.F():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.F if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.F=v end return v.c end end do local function __modImpl()local Element = __DARKLUA_BUNDLE_MODULES.d()
.miu.Element

local theme = __DARKLUA_BUNDLE_MODULES.q()

-- boolean: Studio-style checkbox + dim true/false text.
local function BoolValue(id: string, value: boolean)
	Element("Frame", {
		id = id,
		Size = UDim2.new(1, 0, 0, 14),
		tags = { "bg-none", "flex-row", "gap-sm" },
	}, function()
		Element("TextLabel", {
			id = "box",
			Text = if value then "✓" else "",
			Size = UDim2.new(0, 12, 0, 12),
			tags = { "corner-sm", "vs-border" },
			BackgroundColor3 = if value then theme.custom else theme.surfaceHigh,
			TextColor3 = theme.bg,
			TextSize = 10,
			Font = theme.font,
		})
		Element("TextLabel", {
			id = "text",
			Text = tostring(value),
			Size = UDim2.new(1, -18, 0, 14),
			tags = { "text", "text-small", "muted" },
		})
	end)
end

return BoolValue
end function __DARKLUA_BUNDLE_MODULES.G():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.G if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.G=v end return v.c end end do local function __modImpl()local Element = __DARKLUA_BUNDLE_MODULES.d()
.miu.Element

-- Color3: swatch + friendly RGB ints (the precise float triple stays in the
-- comparison card via format.formatValue).
local function ColorValue(id: string, color: Color3)
	Element("Frame", {
		id = id,
		Size = UDim2.new(1, 0, 0, 14),
		tags = { "bg-none", "flex-row", "gap-sm" },
	}, function()
		Element("Frame", {
			id = "swatch",
			Size = UDim2.new(0, 12, 0, 12),
			tags = { "corner-sm", "vs-border" },
			BackgroundColor3 = color,
		})
		Element("TextLabel", {
			id = "text",
			Text = `{math.round(color.R * 255)}, {math.round(color.G * 255)}, {math.round(color.B * 255)}`,
			Size = UDim2.new(1, -18, 0, 14),
			tags = { "text", "text-small" },
		})
	end)
end

return ColorValue
end function __DARKLUA_BUNDLE_MODULES.H():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.H if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.H=v end return v.c end end do local function __modImpl()local Element = __DARKLUA_BUNDLE_MODULES.d()
.miu.Element

local classIcon = __DARKLUA_BUNDLE_MODULES.y()
local format = __DARKLUA_BUNDLE_MODULES.g()

-- Instance reference: class icon + name (e.g. PrimaryPart).
local function InstanceValue(id: string, inst: Instance)
	local icon = classIcon(inst.ClassName)
	Element("Frame", {
		id = id,
		Size = UDim2.new(1, 0, 0, 14),
		tags = { "bg-none", "flex-row", "gap-sm" },
	}, function()
		Element("ImageLabel", {
			id = "icon",
			Image = icon.Image,
			ImageRectOffset = icon.ImageRectOffset,
			ImageRectSize = icon.ImageRectSize,
			Size = UDim2.new(0, 14, 0, 14),
			BackgroundTransparency = 1,
		})
		Element("TextLabel", {
			id = "text",
			RichText = true,
			Text = format.escapeRich(inst.Name),
			Size = UDim2.new(1, -20, 0, 14),
			tags = { "text", "text-small" },
			TextTruncate = Enum.TextTruncate.AtEnd,
		})
	end)
end

return InstanceValue
end function __DARKLUA_BUNDLE_MODULES.I():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.I if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.I=v end return v.c end end do local function __modImpl()local Element = __DARKLUA_BUNDLE_MODULES.d()
.miu.Element

local theme = __DARKLUA_BUNDLE_MODULES.q()

-- Fallback value renderer: mono text, single line. Truncates rather than
-- wraps — rows live in fixed-height virtual list slots (full values return
-- with the comparison card).
local function TextValue(id: string, text: string, color: Color3?)
	Element("TextLabel", {
		id = id,
		RichText = true,
		Text = text,
		Size = UDim2.new(1, 0, 0, 14),
		tags = { "text", "text-small" },
		TextColor3 = color or theme.text,
		TextTruncate = Enum.TextTruncate.AtEnd,
	})
end

return TextValue
end function __DARKLUA_BUNDLE_MODULES.J():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.J if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.J=v end return v.c end end do local function __modImpl()--[[
	Typed value renderer: dispatches a property value to its dedicated
	component — color swatches, checkboxes, instance refs with class icons,
	texture previews — with mono text as the fallback for everything else.
]]

local format = __DARKLUA_BUNDLE_MODULES.g()
local theme = __DARKLUA_BUNDLE_MODULES.q()

local AssetValue = __DARKLUA_BUNDLE_MODULES.F()
local BoolValue = __DARKLUA_BUNDLE_MODULES.G()
local ColorValue = __DARKLUA_BUNDLE_MODULES.H()
local InstanceValue = __DARKLUA_BUNDLE_MODULES.I()
local TextValue = __DARKLUA_BUNDLE_MODULES.J()

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
end function __DARKLUA_BUNDLE_MODULES.K():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.K if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.K=v end return v.c end end do local function __modImpl()
local packages = __DARKLUA_BUNDLE_MODULES.d()
local Element = packages.miu.Element
local tags = packages.wind.tags

local PropertyValue = __DARKLUA_BUNDLE_MODULES.K()

-- One property: name column (prop-blue) + the typed value renderer.
-- stripe = zebra shading.
local function PropertyRow(id: string, name: string, value: any, propertyName: string?, stripe: boolean?)
	Element("Frame", {
		id = id,
		tags = tags("row", "auto-size-y", "flex-row", "gap-sm", stripe and "stripe"),
	}, function()
		Element("TextLabel", {
			id = "name",
			Text = name,
			Size = UDim2.new(0, 104, 0, 14),
			tags = { "text", "text-small", "prop-name" },
			TextTruncate = Enum.TextTruncate.AtEnd,
		})
		Element("Frame", {
			id = "value",
			Size = UDim2.new(1, -112, 0, 0),
			AutomaticSize = Enum.AutomaticSize.Y,
			tags = { "bg-none", "flex-col" },
		}, function()
			PropertyValue("typed", value, propertyName)
		end)
	end)
end

return PropertyRow
end function __DARKLUA_BUNDLE_MODULES.L():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.L if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.L=v end return v.c end end do local function __modImpl()--[[
	Plain properties pane for the selected instance: prop names in #9cdcfe,
	typed value renderers, with attributes and tags in their own headed
	sections below the properties. No diff presentation yet — the conflict
	layers come back iteratively. One piece of conflict knowledge stays,
	silently: on the BASE tab a contested property shows the boot snapshot,
	since the live instance may be previewing a staged side.

	Rows render through VirtualList: the pane flattens to a positioned item
	array (section headers + prop rows + tag rows, each with a precomputed
	y/height) and only the visible slice builds UI.

	Studio offers no runtime property enumeration, so the listing is a curated
	per-class set (pcall-guarded) rather than exhaustive — attributes and tags
	ARE enumerable and complete.
]]


local packages = __DARKLUA_BUNDLE_MODULES.d()
local Element = packages.miu.Element
local VirtualList = packages.virtualList.VirtualList
local Dimensions = packages.virtualList.Dimensions
local tags = packages.wind.tags

local format = __DARKLUA_BUNDLE_MODULES.g()

local conflicts = __DARKLUA_BUNDLE_MODULES.j()
local cts = __DARKLUA_BUNDLE_MODULES.i()
local world = __DARKLUA_BUNDLE_MODULES.h()

local PropertyRow = __DARKLUA_BUNDLE_MODULES.L()

local CLASS_PROPS: { { class: string, props: { string } } } = {
	{
		class = "BasePart",
		props = {
			"Position",
			"Orientation",
			"Size",
			"Color",
			"Material",
			"Transparency",
			"Reflectance",
			"Anchored",
			"CanCollide",
		},
	},
	{ class = "Model", props = { "PrimaryPart" } },
	{ class = "Light", props = { "Brightness", "Color", "Enabled" } },
	{ class = "Camera", props = { "CFrame", "Focus", "FieldOfView" } },
	{ class = "Decal", props = { "Texture", "Color3", "Transparency" } },
	{ class = "Sound", props = { "SoundId", "Volume", "PlaybackSpeed" } },
	{ class = "ValueBase", props = { "Value" } },
}

local HEADER_HEIGHT = 24
local SECTION_HEIGHT = 20
local ROW_HEIGHT = 18 -- 14px row + the old gap-sm breathing room
local ASSET_ROW_HEIGHT = 36 -- 32px thumbnail row

-- Flatten the pane into positioned items (prefix-summed y offsets), zebra
-- restarting per section.










local function buildItems(inst: Instance): { Item__DARKLUA_TYPE_af }
	local items: { Item__DARKLUA_TYPE_af } = {}
	local offset = 4
	local rowIndex = 0

	local function push(kind: "section" | "prop" | "tag", h: number, name: string, value: any, propertyName: string?)
		local stripe = false
		if kind == "section" then
			rowIndex = 0
		else
			rowIndex += 1
			stripe = rowIndex % 2 == 0
		end
		table.insert(items, {
			y = offset,
			h = h,
			kind = kind,
			name = name,
			value = value,
			propertyName = propertyName,
			stripe = stripe,
		})
		offset += h
	end

	local function pushProp(name: string, value: any, propertyName: string?)
		local h = if format.isVisualAsset(propertyName, value) then ASSET_ROW_HEIGHT else ROW_HEIGHT
		push("prop", h, name, value, propertyName)
	end

	-- BASE tab fidelity: the live target may be previewing a staged side;
	-- its contested property shows the boot snapshot instead.
	local contestedProperty: string? = nil
	local contestedBaseValue: any = nil
	if world:get(cts.ViewTab, cts.ViewTab) == "base" then
		for _, entity in conflicts.list() do
			if world:get(entity, cts.Target) == inst then
				if world:get(entity, cts.Kind) == "Property" then
					contestedProperty = world:get(entity, cts.Property) :: string?
					local base = world:get(entity, cts.Base) :: Base__DARKLUA_TYPE_G?
					contestedBaseValue = if base then base.value else nil
				end
				break
			end
		end
	end

	pushProp("ClassName", inst.ClassName)
	pushProp("Name", if contestedProperty == "Name" then contestedBaseValue else inst.Name)

	local seen: { [string]: boolean } = { Name = true, ClassName = true }
	for _, group in CLASS_PROPS do
		if inst:IsA(group.class) then
			for _, property in group.props do
				if not seen[property] then
					seen[property] = true
					local ok, value = format.readProperty(inst, property)
					if ok then
						if contestedProperty == property then
							value = contestedBaseValue
						end
						pushProp(property, value, property)
					end
				end
			end
		end
	end

	-- Attribute rows sort by name so the per-frame render is order-stable.
	local attributes = inst:GetAttributes()
	local attributeNames: { string } = {}
	for attribute in attributes do
		table.insert(attributeNames, attribute)
	end
	if #attributeNames > 0 then
		table.sort(attributeNames)
		push("section", SECTION_HEIGHT, "ATTRIBUTES", nil)
		for _, attribute in attributeNames do
			pushProp(attribute, attributes[attribute])
		end
	end

	local instanceTags = inst:GetTags()
	if #instanceTags > 0 then
		table.sort(instanceTags)
		push("section", SECTION_HEIGHT, "TAGS", nil)
		for _, tag in instanceTags do
			push("tag", ROW_HEIGHT, tag, nil)
		end
	end

	return items
end

local itemDimensions = Dimensions.getter(function(item: Item__DARKLUA_TYPE_af)
	return {
		size = UDim2.new(1, 0, 0, item.h),
		position = UDim2.fromOffset(0, item.y),
	}
end)

local function renderItem(item: Item__DARKLUA_TYPE_af)
	if item.kind == "section" then
		Element("TextLabel", {
			id = "header",
			Text = item.name,
			Size = UDim2.fromScale(1, 1),
			tags = { "header-band" },
		})
		return
	end

	Element("UIPadding", {
		id = "pad",
		PaddingLeft = UDim.new(0, 4),
		PaddingRight = UDim.new(0, 4),
	})
	-- Slot children need distinct ids per kind: VirtualList reuses slots
	-- across items, and miu reuses children by id — a Frame left by a prop
	-- row must not be handed TextLabel props when the slot becomes a tag row.
	if item.kind == "prop" then
		PropertyRow("prop", item.name, item.value, item.propertyName, item.stripe)
	else
		Element("TextLabel", {
			id = "tag",
			Text = item.name,
			Size = UDim2.new(1, 0, 0, 14),
			tags = tags("row", "text", "text-small", item.stripe and "stripe"),
			TextTruncate = Enum.TextTruncate.AtEnd,
		})
	end
end

local function PropertyPanel()
	Element("Frame", {
		id = "properties",
		Size = UDim2.fromScale(1, 1),
		tags = { "bg-none", "flex-col" },
	}, function()
		local inst = world:get(cts.SelectedInstance, cts.SelectedInstance) :: Instance?

		Element("TextLabel", {
			id = "title",
			Text = if inst then `PROPERTIES — {inst.Name}` else "PROPERTIES",
			Size = UDim2.new(1, 0, 0, HEADER_HEIGHT),
			tags = { "header-band" },
			TextTruncate = Enum.TextTruncate.AtEnd,
		})

		if world:has(cts.Finished, cts.Finished) or not inst or not inst.Parent then
			return
		end

		debug.profilebegin("gitrbx_props_items")
		local items = buildItems(inst)
		debug.profileend()

		VirtualList({
			id = "rows",
			data = items,
			dimensions = itemDimensions,
			direction = "y",
			Size = UDim2.new(1, 0, 1, -HEADER_HEIGHT),
			tags = { "bg-none" },
		}, function(item: Item__DARKLUA_TYPE_af)
			renderItem(item)
		end)
	end)
end

return PropertyPanel
end function __DARKLUA_BUNDLE_MODULES.M():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.M if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.M=v end return v.c end end do local function __modImpl()-- Selected-instance properties dock widget, synchronized through the ECS world.


local packages = __DARKLUA_BUNDLE_MODULES.d()
local Element = packages.miu.Element
local Portal = packages.miu.Portal

local theme = __DARKLUA_BUNDLE_MODULES.q()

local types = __DARKLUA_BUNDLE_MODULES.m()
local widgets = __DARKLUA_BUNDLE_MODULES.x()

local PropertyPanel = __DARKLUA_BUNDLE_MODULES.M()

return function(_world: World__DARKLUA_TYPE_P, _cts: Cts__DARKLUA_TYPE_Q, scheduler: Scheduler__DARKLUA_TYPE_R)
	local widget = widgets.create("GitRbxMergeProperties", "git-rbx — properties", Enum.InitialDockState.Left, {
		width = 340,
		height = 500,
		minWidth = 240,
		minHeight = 180,
	})

	scheduler:addSystem({
		name = "ui_properties",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			debug.profilebegin("gitrbx_ui_properties")
			Portal(widget, "git-rbx-properties", function()
				theme.link()
				Element("Frame", {
					id = "properties-widget",
					Size = UDim2.fromScale(1, 1),
					tags = { "surface", "vs-border" },
				}, function()
					PropertyPanel()
				end)
			end)
			debug.profileend()
		end,
	})
end
end function __DARKLUA_BUNDLE_MODULES.N():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.N if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.N=v end return v.c end end do local function __modImpl()--[[
	One-glance descriptions of a conflict's sides for the conflict list and
	the comparison card. Resolver-only: these read the stamped SideInfo /
	Base components, which the diff viewer never has.
]]


local cts = __DARKLUA_BUNDLE_MODULES.i()
local format = __DARKLUA_BUNDLE_MODULES.g()




local summary = {}

-- What choosing this side means, one glance: the property value it sets, the
-- destination it reparents to, or whether it keeps (edited) / deletes the target.
function summary.side(kind: string, property: string?, info: SideInfo__DARKLUA_TYPE_ag?): string?
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
function summary.base(kind: string, base: Base__DARKLUA_TYPE_ah): string
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
end function __DARKLUA_BUNDLE_MODULES.O():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.O if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.O=v end return v.c end end do local function __modImpl()
local Element = __DARKLUA_BUNDLE_MODULES.d()
.miu.Element

local theme = __DARKLUA_BUNDLE_MODULES.q()
local Icon = __DARKLUA_BUNDLE_MODULES.B()

local function Button(
	id: string,
	text: string,
	highlighted: boolean,
	color: Color3,
	onClick: () -> (),
	onView: (() -> ())?
)
	if not onView then
		Element("TextButton", {
			id = id,
			Text = text,
			Size = UDim2.fromOffset(64, 22),
			tags = { "corner-sm", "button" },
			BackgroundColor3 = if highlighted then color else theme.neutralButton,
			TextColor3 = if highlighted then theme.bg else theme.text,
			AutoButtonColor = true,
			events = { Activated = onClick },
		})
		return
	end

	-- Compound choice: the wide section stages/toggles the resolution, while
	-- the explicit View section stages this side without toggling and frames it.
	-- They are siblings inside one visual control rather than nested buttons,
	-- so activating View cannot bubble into the choice handler.
	Element("Frame", {
		id = id,
		Size = UDim2.fromOffset(90, 22),
		tags = { "bg-none", "flex-row", "gap-xs" },
	}, function()
		Element("TextButton", {
			id = "choose",
			Text = text,
			Size = UDim2.fromOffset(64, 22),
			tags = { "corner-sm", "button" },
			BackgroundColor3 = if highlighted then color else theme.neutralButton,
			TextColor3 = if highlighted then theme.bg else theme.text,
			AutoButtonColor = true,
			events = { Activated = onClick },
		})
		Element("TextButton", {
			id = "view",
			Text = "View",
			TextTransparency = 1,
			Size = UDim2.fromOffset(22, 22),
			tags = { "corner-sm", "button" },
			BackgroundColor3 = theme.neutralButton,
			AutoButtonColor = true,
			events = { Activated = onView },
		}, function()
			Icon("icon", "lucide:eye", color, UDim2.fromOffset(14, 14))
		end)
	end)
end

return Button
end function __DARKLUA_BUNDLE_MODULES.P():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.P if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.P=v end return v.c end end do local function __modImpl()local Element = __DARKLUA_BUNDLE_MODULES.d()
.miu.Element

local theme = __DARKLUA_BUNDLE_MODULES.q()

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
end function __DARKLUA_BUNDLE_MODULES.Q():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.Q if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.Q=v end return v.c end end do local function __modImpl()local packages = __DARKLUA_BUNDLE_MODULES.d()
local Element = packages.miu.Element

local format = __DARKLUA_BUNDLE_MODULES.g()
local summary = __DARKLUA_BUNDLE_MODULES.O()
local theme = __DARKLUA_BUNDLE_MODULES.q()

local conflicts = __DARKLUA_BUNDLE_MODULES.j()
local cts = __DARKLUA_BUNDLE_MODULES.i()
local world = __DARKLUA_BUNDLE_MODULES.h()

local Button = __DARKLUA_BUNDLE_MODULES.P()
local Label = __DARKLUA_BUNDLE_MODULES.Q()

local function hasSpatialBounds(target: Instance): boolean
	if target:IsA("PVInstance") then
		return true
	end
	for _, descendant in target:GetDescendants() do
		if descendant:IsA("BasePart") then
			return true
		end
	end
	return false
end

-- One conflict — or one rigid group, which reads member state and stages
-- every member at once.
local function ConflictRow(entity: any)
	local kind = world:get(entity, cts.Kind) :: string
	local path = world:get(entity, cts.Path) :: string
	local property = world:get(entity, cts.Property) :: string?
	local target = world:get(entity, cts.Target) :: Instance?
	local members = world:get(entity, cts.Members) :: { any }?
	local resolution = if members
		then conflicts.groupResolution(entity)
		else world:get(entity, cts.Resolution) :: string?
	local selected = world:get(cts.SelectedConflict, cts.SelectedConflict) == entity
	local viewable = target ~= nil and target:IsDescendantOf(workspace) and hasSpatialBounds(target)

	local function requestFocus()
		if viewable then
			world:set(world:entity(), cts.FocusRequest, entity)
		end
	end

	local function selectTarget()
		conflicts.select(entity)
	end

	local function choose(side: string)
		if members then
			conflicts.chooseGroup(entity, side)
		else
			conflicts.choose(entity, side)
		end
	end

	-- Viewing is deliberately explicit and non-toggling. It stages the named
	-- side so the scene contains that version, then the next frame focuses the
	-- content after renderPreviews has applied it.
	local function view(side: string)
		if members then
			conflicts.stageGroup(entity, side)
		else
			conflicts.stage(entity, side)
		end
		requestFocus()
	end

	-- Fixed-height card that fills its VirtualList slot; every line truncates
	-- instead of wrapping so the row height stays uniform.
	Element("TextButton", {
		id = "card",
		Text = "",
		AutoButtonColor = false,
		Size = UDim2.fromScale(1, 1),
		tags = { "flex-col", "gap-sm", "p-md", "corner-sm", "vs-border" },
		BackgroundColor3 = if selected then theme.surface:Lerp(theme.selection, 0.35) else theme.surface,
		events = {
			Activated = selectTarget,
		},
	}, function()
		Label("path", path, theme.statusColor(resolution), true)

		local detail = if members
			then `{kind} — {#members} spatial changes`
			else kind .. (if property then ` ({property})` else "")
		Element("TextLabel", {
			id = "detail",
			Text = detail,
			tags = { "text", "text-small", "muted", "auto-size-y" },
			TextTruncate = Enum.TextTruncate.AtEnd,
		})

		local oursSummary: string?
		local theirsSummary: string?
		if members then
			local deltaOurs = world:get(entity, cts.DeltaOurs) :: CFrame?
			local deltaTheirs = world:get(entity, cts.DeltaTheirs) :: CFrame?
			oursSummary = if deltaOurs then format.formatDelta(deltaOurs) else nil
			theirsSummary = if deltaTheirs then format.formatDelta(deltaTheirs) else nil
		else
			oursSummary = summary.side(kind, property, world:get(entity, cts.Ours))
			theirsSummary = summary.side(kind, property, world:get(entity, cts.Theirs))
		end
		if oursSummary or theirsSummary then
			Element("Frame", {
				id = "values",
				Size = UDim2.new(1, 0, 0, 28),
				tags = { "bg-none", "flex-col" },
			}, function()
				Element("TextLabel", {
					id = "ours",
					RichText = true,
					Text = oursSummary or "?",
					Size = UDim2.new(1, 0, 0, 14),
					tags = { "text", "text-small" },
					TextColor3 = theme.ours,
					TextTruncate = Enum.TextTruncate.AtEnd,
				})
				Element("TextLabel", {
					id = "theirs",
					RichText = true,
					Text = theirsSummary or "?",
					Size = UDim2.new(1, 0, 0, 14),
					tags = { "text", "text-small" },
					TextColor3 = theme.theirs,
					TextTruncate = Enum.TextTruncate.AtEnd,
				})
			end)
		end

		Element("Frame", {
			id = "buttons",
			Size = UDim2.new(1, 0, 0, 22),
			tags = { "bg-none", "flex-row", "gap-sm" },
		}, function()
			Button(
				"ours",
				"Ours",
				resolution == "ours",
				theme.ours,
				function()
					choose("ours")
				end,
				if viewable
					then function()
						view("ours")
					end
					else nil
			)
			Button(
				"theirs",
				"Theirs",
				resolution == "theirs",
				theme.theirs,
				function()
					choose("theirs")
				end,
				if viewable
					then function()
						view("theirs")
					end
					else nil
			)
			if not members and conflicts.canCustom(entity) then
				Button("custom", "Custom", resolution == "custom", theme.custom, function()
					choose("custom")
				end)
			end
			Button("select", "Select", selected, theme.selection, selectTarget)
		end)
	end)
end

return ConflictRow
end function __DARKLUA_BUNDLE_MODULES.R():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.R if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.R=v end return v.c end end do local function __modImpl()
local packages = __DARKLUA_BUNDLE_MODULES.d()
local Element = packages.miu.Element
local VirtualList = packages.virtualList.VirtualList
local Dimensions = packages.virtualList.Dimensions

local theme = __DARKLUA_BUNDLE_MODULES.q()
local transport = __DARKLUA_BUNDLE_MODULES.k()

local conflicts = __DARKLUA_BUNDLE_MODULES.j()
local cts = __DARKLUA_BUNDLE_MODULES.i()
local world = __DARKLUA_BUNDLE_MODULES.h()

local ConflictRow = __DARKLUA_BUNDLE_MODULES.R()

local HEADER_HEIGHT = 24
local CARD_HEIGHT = 106
local CARD_GAP = 4
local FOOTER_HEIGHT = 36

-- The conflict list + Complete button. Cards render through VirtualList —
-- only the visible slice builds ConflictRows, so real merges (1000+
-- conflicts) stay interactive. Complete lives in a pinned footer instead of
-- after the last card.
local function Panel()
	local cards = conflicts.cards()
	debug.profilebegin("gitrbx_panel_count")
	local unresolved = 0
	for _, entity in cards do
		local members = world:get(entity, cts.Members) :: { any }?
		local resolution = if members then conflicts.groupResolution(entity) else world:get(entity, cts.Resolution)
		if resolution == nil then
			unresolved += 1
		end
	end
	debug.profileend()

	local finished = world:has(cts.Finished, cts.Finished)
	local completing = world:has(cts.Completing, cts.Completing)
	local completeError = world:get(cts.CompleteError, cts.CompleteError) :: string?

	Element("Frame", {
		id = "panel",
		Size = UDim2.fromScale(1, 1),
		tags = { "bg-none", "flex-col" },
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = if finished
				then "MERGE COMPLETE — FILE WRITTEN"
				else `CONFLICTS — {#cards - unresolved}/{#cards} DECIDED`,
			Size = UDim2.new(1, 0, 0, HEADER_HEIGHT),
			tags = { "header-band" },
			TextColor3 = if finished then theme.added else theme.textMuted,
		})

		if finished then
			return
		end

		debug.profilebegin("gitrbx_panel_cards")
		VirtualList({
			id = "cards",
			data = cards,
			dimensions = Dimensions.withSpacing(Dimensions.consistentSize(CARD_HEIGHT), CARD_GAP),
			direction = "y",
			Size = UDim2.new(1, 0, 1, -HEADER_HEIGHT - FOOTER_HEIGHT),
			tags = { "bg-none" },
		}, function(entity: any)
			Element("UIPadding", {
				id = "pad",
				PaddingLeft = UDim.new(0, 4),
				PaddingRight = UDim.new(0, 4),
			})
			ConflictRow(entity)
		end)
		debug.profileend()

		Element("Frame", {
			id = "footer",
			Size = UDim2.new(1, 0, 0, FOOTER_HEIGHT),
			tags = { "bg-none", "flex-col", "p-sm" },
		}, function()
			local canComplete = unresolved == 0 and not completing
			Element("TextButton", {
				id = "complete",
				Text = if completing
					then "Completing…"
					elseif completeError then "Complete failed — retry"
					elseif unresolved > 0 then `Complete ({unresolved} undecided)`
					else "Complete",
				Size = UDim2.fromScale(1, 1),
				tags = { "corner-sm", "button" },
				BackgroundColor3 = if canComplete then theme.accent else theme.disabled,
				TextColor3 = if canComplete then theme.textBright else theme.textMuted,
				TextSize = theme.textSize,
				events = {
					Activated = function()
						if canComplete then
							transport.complete()
						end
					end,
				},
			})
		end)
	end)
end

return Panel
end function __DARKLUA_BUNDLE_MODULES.S():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.S if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.S=v end return v.c end end do local function __modImpl()--[[
	Conflicts dock widget: the conflict list + Complete, rendered
	immediate-mode every heartbeat.
]]


local packages = __DARKLUA_BUNDLE_MODULES.d()
local Element = packages.miu.Element
local Portal = packages.miu.Portal

local theme = __DARKLUA_BUNDLE_MODULES.q()

local types = __DARKLUA_BUNDLE_MODULES.m()
local widgets = __DARKLUA_BUNDLE_MODULES.x()

local Panel = __DARKLUA_BUNDLE_MODULES.S()

return function(_world: World__DARKLUA_TYPE_P, _cts: Cts__DARKLUA_TYPE_Q, scheduler: Scheduler__DARKLUA_TYPE_R)
	local widget = widgets.create("GitRbxMergeConflicts", "git-rbx — conflicts", Enum.InitialDockState.Right, {
		width = 460,
		height = 600,
		minWidth = 320,
		minHeight = 260,
	})

	scheduler:addSystem({
		name = "ui_conflicts",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			debug.profilebegin("gitrbx_ui_conflicts")
			Portal(widget, "git-rbx-conflicts", function()
				debug.profilebegin("gitrbx_conflicts_body")
				theme.link()
				Element("Frame", {
					id = "conflicts",
					Size = UDim2.fromScale(1, 1),
					tags = { "surface", "vs-border" },
				}, function()
					Panel()
				end)
				debug.profileend()
			end)
			debug.profileend()
		end,
	})
end
end function __DARKLUA_BUNDLE_MODULES.T():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.T if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.T=v end return v.c end end do local function __modImpl()--[[
	One cell of a change list, after Rojo's DisplayValue: colors get a swatch
	plus `r, g, b`, strings are quoted, everything else is the typed value's
	text form. Accepts either Rust's serialized PropertyValue or a RichText
	override.
]]


local Element = __DARKLUA_BUNDLE_MODULES.d()
.miu.Element

local format = __DARKLUA_BUNDLE_MODULES.g()
local theme = __DARKLUA_BUNDLE_MODULES.q()

local function colorOf(value: any): Color3?
	if type(value) == "table" and value.type == "color3" and value.value then
		local v = value.value
		return Color3.new(v.r, v.g, v.b)
	end
	return nil
end

local function DisplayValue(id: string, value: any, text: string?, color: Color3)
	local swatch = if text == nil then colorOf(value) else nil
	local label = text
		or (
			if swatch
				then string.format("%d, %d, %d", swatch.R * 255, swatch.G * 255, swatch.B * 255)
				else format.patchValue(value)
		)

	Element("Frame", {
		id = id,
		Size = UDim2.fromScale(1, 1),
		BackgroundTransparency = 1,
	}, function()
		if swatch then
			Element("Frame", {
				id = "swatch",
				Size = UDim2.fromOffset(16, 16),
				Position = UDim2.fromScale(0, 0.5),
				AnchorPoint = Vector2.new(0, 0.5),
				BackgroundColor3 = swatch,
				BackgroundTransparency = 0,
				BorderSizePixel = 0,
			}, function()
				Element("UICorner", { id = "corner", CornerRadius = UDim.new(0, 4) })
				Element("UIStroke", { id = "stroke", Color = theme.border, Thickness = 1 })
			end)
		end
		Element("TextLabel", {
			id = "label",
			RichText = true,
			Text = label,
			Position = UDim2.fromOffset(if swatch then 21 else 0, 0),
			Size = UDim2.new(1, if swatch then -21 else 0, 1, 0),
			tags = { "text", "text-small" },
			TextColor3 = color,
			TextTruncate = Enum.TextTruncate.AtEnd,
		})
	end)
end

return DisplayValue
end function __DARKLUA_BUNDLE_MODULES.U():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.U if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.U=v end return v.c end end do local function __modImpl()











































































return {}
end function __DARKLUA_BUNDLE_MODULES.V():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.V if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.V=v end return v.c end end do local function __modImpl()--[[
	The expanded change table under a node, after Rojo's ChangeList: a bold
	header row then one row per change, Property / Current / Incoming at
	30 / 35 / 35 percent, indented under the node. Rows are rendered one at
	a time by the visualizer's virtual list, so this module exposes the two
	row renderers rather than a whole table.
]]


local Element = __DARKLUA_BUNDLE_MODULES.d()
.miu.Element
local tags = __DARKLUA_BUNDLE_MODULES.d()
.wind.tags

local DisplayValue = __DARKLUA_BUNDLE_MODULES.U()
local format = __DARKLUA_BUNDLE_MODULES.g()
local theme = __DARKLUA_BUNDLE_MODULES.q()
local types = __DARKLUA_BUNDLE_MODULES.V()

local ChangeList = {}



local function columns(row: Row__DARKLUA_TYPE_as, callback: () -> ())
	-- Rojo indents the table by the node's own indent; the node row puts
	-- its class icon at indent + 2, so the table starts at the icon.
	local indent = (row.node.depth - 1) * 12 + 25
	Element("Frame", {
		id = "columns",
		Position = UDim2.fromOffset(indent, 0),
		Size = UDim2.new(1, -indent - 6, 1, 0),
		BackgroundTransparency = 1,
	}, function()
		Element("UIListLayout", {
			id = "layout",
			FillDirection = Enum.FillDirection.Horizontal,
			SortOrder = Enum.SortOrder.LayoutOrder,
			VerticalAlignment = Enum.VerticalAlignment.Center,
			Padding = UDim.new(0, 4),
		})
		callback()
	end)
end

local function cell(id: string, width: number, order: number, callback: () -> ())
	Element("Frame", {
		id = id,
		Size = UDim2.fromScale(width, 1),
		LayoutOrder = order,
		BackgroundTransparency = 1,
	}, callback)
end

local function headerCell(id: string, text: string, width: number, order: number)
	Element("TextLabel", {
		id = id,
		Text = text,
		Size = UDim2.fromScale(width, 1),
		LayoutOrder = order,
		tags = { "text", "text-small" },
		TextColor3 = theme.text,
		FontFace = theme.fontFaceBold,
		TextTruncate = Enum.TextTruncate.AtEnd,
	})
end

function ChangeList.renderHeader(row: Row__DARKLUA_TYPE_as, headers: Headers__DARKLUA_TYPE_at)
	Element("Frame", {
		id = "change-header",
		Size = UDim2.fromScale(1, 1),
		tags = tags("row", row.index % 2 == 0 and "stripe"),
	}, function()
		columns(row, function()
			headerCell("property", headers[1], 0.3, 1)
			headerCell("current", headers[2], 0.35, 2)
			headerCell("incoming", headers[3], 0.35, 3)
		end)
	end)
end

function ChangeList.renderChange(row: Row__DARKLUA_TYPE_as, onActivate: (() -> ())?)
	local change = row.change :: ChangeRow__DARKLUA_TYPE_al	
local warning = change.warning == true
	local valueColor = if warning then theme.warning else theme.textMuted
	Element("TextButton", {
		id = "change",
		Text = "",
		Size = UDim2.fromScale(1, 1),
		AutoButtonColor = false,
		tags = tags("row", row.index % 2 == 0 and "stripe"),
		events = {
			Activated = function()
				if onActivate then
					onActivate()
				end
			end,
		},
	}, function()
		columns(row, function()
			Element("TextLabel", {
				id = "property",
				RichText = true,
				Text = (if warning then "⚠ " else "") .. format.escapeRich(change.property),
				Size = UDim2.fromScale(0.3, 1),
				LayoutOrder = 1,
				tags = { "text", "text-small" },
				TextColor3 = if warning then theme.warning else theme.text,
				TextTruncate = Enum.TextTruncate.AtEnd,
			})
			cell("current", 0.35, 2, function()
				DisplayValue("value", change.current, change.currentText, valueColor)
			end)
			cell("incoming", 0.35, 3, function()
				DisplayValue("value", change.incoming, change.incomingText, valueColor)
			end)
		end)
	end)
end

return ChangeList
end function __DARKLUA_BUNDLE_MODULES.W():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.W if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.W=v end return v.c end end do local function __modImpl()--[[
	One instance row, after Rojo's DomLabel: line guides per depth with a
	connector, a kind marker at the far left, the class icon, the name in
	the kind's color (bold when changed, plain for ancestors), and the number
	of changes right-aligned. Single click toggles the change list, double
	click selects the instance.
]]


local Element = __DARKLUA_BUNDLE_MODULES.d()
.miu.Element
local tags = __DARKLUA_BUNDLE_MODULES.d()
.wind.tags

local Icon = __DARKLUA_BUNDLE_MODULES.B()
local classIcon = __DARKLUA_BUNDLE_MODULES.y()
local format = __DARKLUA_BUNDLE_MODULES.g()
local theme = __DARKLUA_BUNDLE_MODULES.q()
local types = __DARKLUA_BUNDLE_MODULES.V()



local ICON_BY_KIND = {
	Add = "lucide:plus",
	Remove = "lucide:minus",
	Edit = "lucide:pencil",
	Reparent = "lucide:move-right",
	Pivot = "lucide:rotate-3d",
}

local DomLabel = {}

function DomLabel.colorOf(kind: NodeKind__DARKLUA_TYPE_am?, warning: boolean?): Color3
	if warning then
		return theme.warning
	elseif kind == "Add" then
		return theme.added
	elseif kind == "Remove" then
		return theme.removed
	elseif kind == "Edit" then
		return theme.modified
	elseif kind == "Reparent" then
		return theme.reparented
	elseif kind == "Pivot" then
		return theme.pivoted
	end
	return theme.text
end










local lastDoubleClick: { [Id__DARKLUA_TYPE_aw]: number } = {}

function DomLabel.render(props: Props__DARKLUA_TYPE_ax)
	local row = props.row
	local node = row.node
	local depth = node.depth
	local color = DomLabel.colorOf(node.kind, node.warning)
	local indent = (depth - 1) * 12 + 15
	local hasChanges = #node.rows > 0
	local half = props.rowHeight / 2

	Element("Frame", {
		id = "node",
		Size = UDim2.fromScale(1, 1),
		ClipsDescendants = true,
		tags = tags("row", row.index % 2 == 0 and "stripe", props.selected and "selected"),
	}, function()
		Element("UIPadding", {
			id = "padding",
			PaddingLeft = UDim.new(0, 10),
			PaddingRight = UDim.new(0, 10),
		})

		-- Line guides: one per ancestor depth still in progress; the final
		-- child's own guide stops halfway to meet the connector.
		for i = 2, depth do
			if row.depthsComplete[i] then
				continue
			end
			local short = node.isFinalChild and i == depth
			Element("Frame", {
				id = `guide-{i}`,
				Size = if short
					then UDim2.fromOffset(2, half + 3)
					else UDim2.new(0, 2, 1, if row.isFinalElement then -(half - 3) else 2),
				Position = UDim2.fromOffset((12 * (i - 1)) + 6, -1),
				BorderSizePixel = 0,
				BackgroundColor3 = theme.border,
				BackgroundTransparency = 0,
			})
		end
		if depth ~= 1 then
			Element("Frame", {
				id = "connector",
				Size = UDim2.fromOffset(8, 2),
				Position = UDim2.fromOffset(2 + (12 * depth), half),
				AnchorPoint = Vector2.new(1, 0),
				BorderSizePixel = 0,
				BackgroundColor3 = theme.border,
				BackgroundTransparency = 0,
			})
		end

		Element("TextButton", {
			id = "button",
			Text = "",
			Size = UDim2.fromScale(1, 1),
			AutoButtonColor = false,
			tags = { "bg-none" },
			events = {
				Activated = function(_input: InputObject, clickCount: number)
					if clickCount == 1 then
						lastDoubleClick[node.id] = os.clock()
						if props.onSelect then
							props.onSelect()
						end
					elseif clickCount == 0 and hasChanges then
						task.delay(0.25, function()
							if os.clock() - (lastDoubleClick[node.id] or 0) <= 0.3 then
								return
							end
							props.onToggle()
						end)
					end
				end,
			},
		})

		if node.kind then
			Element("Frame", {
				id = "diff-icon",
				Size = UDim2.fromOffset(14, 14),
				Position = UDim2.fromScale(0, 0.5),
				AnchorPoint = Vector2.new(0, 0.5),
				BackgroundTransparency = 1,
			}, function()
				Icon("icon", ICON_BY_KIND[node.kind], color, UDim2.fromOffset(14, 14))
			end)
		end

		local icon = classIcon(node.className)
		Element("ImageLabel", {
			id = "class-icon",
			Image = icon.Image,
			ImageRectOffset = icon.ImageRectOffset,
			ImageRectSize = icon.ImageRectSize,
			ImageColor3 = color,
			Size = UDim2.fromOffset(16, 16),
			Position = UDim2.new(0, indent + 2, 0.5, 0),
			AnchorPoint = Vector2.new(0, 0.5),
			BackgroundTransparency = 1,
		})

		Element("TextLabel", {
			id = "name",
			RichText = true,
			Text = (if node.warning then "⚠ " else "") .. format.escapeRich(node.name),
			Position = UDim2.fromOffset(indent + 22, 0),
			Size = UDim2.new(1, -indent - 22 - 50, 1, 0),
			tags = { "text" },
			TextColor3 = color,
			FontFace = if node.kind then theme.fontFaceBold else theme.fontFace,
			TextTruncate = Enum.TextTruncate.AtEnd,
		})

		-- Rojo shows the edit count for edited and added nodes and nothing
		-- for removals; a removed subtree is one row with no children.
		local count = if node.kind ~= "Remove" and hasChanges then #node.rows else nil
		if count then
			Element("TextLabel", {
				id = "count",
				Text = tostring(count),
				Position = UDim2.fromScale(1, 0),
				AnchorPoint = Vector2.new(1, 0),
				Size = UDim2.new(0, 48, 1, 0),
				tags = { "text", "muted" },
				TextXAlignment = Enum.TextXAlignment.Right,
			})
		end
	end)
end

return DomLabel
end function __DARKLUA_BUNDLE_MODULES.X():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.X if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.X=v end return v.c end end do local function __modImpl()--[[
	Pure tree construction after Rojo's PatchTree: nodes hang off their
	parents, siblings sort alphabetically by name at every level, and the
	tree flattens to positioned rows for the virtual list. Expansion state is
	owned by the caller and passed in per flatten.
]]


local types = __DARKLUA_BUNDLE_MODULES.V()







local patchTree = {}

local KIND_RANK = { Add = 0, Remove = 0, Reparent = 1, Pivot = 2, Edit = 3 }

-- Marker precedence when a node carries several kinds of change.
local function deriveKind(input: PatchNodeInput__DARKLUA_TYPE_an): NodeKind__DARKLUA_TYPE_am?
	if input.kind then
		return input.kind
	end
	local best: NodeKind__DARKLUA_TYPE_am? = nil
	for _, row in input.rows or {} do
		local kind: NodeKind__DARKLUA_TYPE_am= if row.kind == "Add" then "Edit" else row.kind
		if not best or KIND_RANK[kind] < KIND_RANK[best] then
			best = kind
		end
	end
	return best
end

local function sortRows(rows: { ChangeRow__DARKLUA_TYPE_aB }): { ChangeRow__DARKLUA_TYPE_aB }
	local sorted = table.clone(rows)
	table.sort(sorted, function(a, b)
		return a.property < b.property
	end)
	return sorted
end

function patchTree.build(patch: Patch__DARKLUA_TYPE_az): { PatchNode__DARKLUA_TYPE_aA }
	local nodes: { [Id__DARKLUA_TYPE_ay]: PatchNode__DARKLUA_TYPE_aA } = {}
	for id, input in patch.nodes do
		nodes[id] = {
			id = id,
			name = input.name,
			className = input.className,
			instance = input.instance,
			kind = deriveKind(input),
			rows = sortRows(input.rows or {}),
			subtreeCount = input.subtreeCount,
			warning = input.warning,
			children = {},
			depth = 1,
			isFinalChild = true,
		}
	end

	local roots: { PatchNode__DARKLUA_TYPE_aA } = {}
	for id, node in nodes do
		local parentId = patch.nodes[id].parent
		local parent = if parentId ~= nil then nodes[parentId] else nil
		if parent then
			node.parent = parent
			table.insert(parent.children, node)
		else
			table.insert(roots, node)
		end
	end

	local function finish(list: { PatchNode__DARKLUA_TYPE_aA }, depth: number)
		table.sort(list, function(a, b)
			if a.name == b.name then
				return tostring(a.id) < tostring(b.id)
			end
			return a.name < b.name
		end)
		for index, node in list do
			node.depth = depth
			node.isFinalChild = index == #list
			finish(node.children, depth + 1)
		end
	end
	finish(roots, 1)
	return roots
end

function patchTree.count(roots: { PatchNode__DARKLUA_TYPE_aA }): number
	local total = 0
	local function visit(node: PatchNode__DARKLUA_TYPE_aA)
		total += 1
		for _, child in node.children do
			visit(child)
		end
	end
	for _, root in roots do
		visit(root)
	end
	return total
end

-- Flatten to rows. `isExpanded(node)` decides whether a node's change list
-- is shown as a header row plus one row per change. `depthsComplete` is a
-- snapshot per row so line guides know which ancestor columns are finished.
function patchTree.flatten(roots: { PatchNode__DARKLUA_TYPE_aA }, rowHeight: number, isExpanded: (PatchNode__DARKLUA_TYPE_aA) -> boolean): { Row__DARKLUA_TYPE_aC }
	local rows: { Row__DARKLUA_TYPE_aC } = {}
	local y = 0
	local depthsComplete: { [number]: boolean } = {}
	local total = patchTree.count(roots)
	local seen = 0

	local function push(kind: RowKind__DARKLUA_TYPE_aq, node: PatchNode__DARKLUA_TYPE_aA, change: ChangeRow__DARKLUA_TYPE_aB?)
		table.insert(rows, {
			y = y,
			h = rowHeight,
			kind = kind,
			index = #rows + 1,
			node = node,
			change = change,
			isFinalElement = seen == total,
			depthsComplete = table.clone(depthsComplete),
		})
		y += rowHeight
	end

	local function visit(node: PatchNode__DARKLUA_TYPE_aA)
		seen += 1
		depthsComplete[node.depth] = false
		for i = node.depth + 1, #depthsComplete do
			depthsComplete[i] = nil
		end
		push("node", node, nil)
		if node.isFinalChild then
			depthsComplete[node.depth] = true
		end
		if #node.rows > 0 and isExpanded(node) then
			push("header", node, nil)
			for _, change in node.rows do
				push("change", node, change)
			end
		end
		for _, child in node.children do
			visit(child)
		end
	end
	for _, root in roots do
		visit(root)
	end
	return rows
end

return patchTree
end function __DARKLUA_BUNDLE_MODULES.Y():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.Y if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.Y=v end return v.c end end do local function __modImpl()--[[
	PatchVisualizer — Rojo's sync-changes tree for git-rbx's five change
	kinds (Add / Remove / Edit / Reparent / Pivot).

	Feed it a neutral Patch (see types.luau) and it renders the sparse tree:
	changed instances plus the ancestors needed to reach them, siblings
	alphabetical, one marker per node, edit counts on the right, and a
	Property / Current / Incoming table under any node you click. Double
	click hands the node to `onSelect`.

	Expansion state is keyed by `stateKey` so a caller showing a different
	patch (another conflict, another side) starts collapsed, as Rojo's rows
	do when their content changes.
]]


local packages = __DARKLUA_BUNDLE_MODULES.d()
local Element = packages.miu.Element
local VirtualList = packages.virtualList.VirtualList
local Dimensions = packages.virtualList.Dimensions

local ChangeList = __DARKLUA_BUNDLE_MODULES.W()
local DomLabel = __DARKLUA_BUNDLE_MODULES.X()
local patchTree = __DARKLUA_BUNDLE_MODULES.Y()
local types = __DARKLUA_BUNDLE_MODULES.V()


















local ROW_HEIGHT = 24
local DEFAULT_HEADERS = { "Property", "Current", "Incoming" }

local expandedFor: any = nil
local expanded: { [Id__DARKLUA_TYPE_aH]: boolean } = {}

local rowDimensions = Dimensions.getter(function(row: Row__DARKLUA_TYPE_ar)
	return {
		size = UDim2.new(1, 0, 0, row.h),
		position = UDim2.fromOffset(0, row.y),
	}
end)

local function PatchVisualizer(props: Props__DARKLUA_TYPE_aI)
	if props.stateKey ~= expandedFor then
		expandedFor = props.stateKey
		table.clear(expanded)
	end

	local roots = patchTree.build(props.patch)
	local rows = patchTree.flatten(roots, ROW_HEIGHT, function(node)
		return expanded[node.id] == true
	end)
	local headers = props.headers or DEFAULT_HEADERS

	Element("Frame", {
		id = "patch-visualizer",
		Size = props.Size or UDim2.fromScale(1, 1),
		Position = props.Position,
		BackgroundTransparency = 1,
	}, function()
		if #rows == 0 then
			Element("TextLabel", {
				id = "empty",
				Text = props.emptyText or "No changes",
				Size = UDim2.fromScale(1, 1),
				tags = { "text", "muted" },
				TextXAlignment = Enum.TextXAlignment.Center,
				TextYAlignment = Enum.TextYAlignment.Center,
			})
			return
		end

		VirtualList({
			id = "rows",
			data = rows,
			dimensions = rowDimensions,
			direction = "y",
			Size = UDim2.fromScale(1, 1),
			tags = { "bg-none" },
		}, function(row: Row__DARKLUA_TYPE_ar)
			local node = row.node
			local select = if props.onSelect
				then function()
					(props.onSelect :: any)(node)
				end
				else nil
			if row.kind == "node" then
				DomLabel.render({
					row = row,
					rowHeight = ROW_HEIGHT,
					expanded = expanded[node.id] == true,
					selected = props.selectedId ~= nil and props.selectedId == node.id,
					onToggle = function()
						expanded[node.id] = not expanded[node.id]
					end,
					onSelect = select,
				})
			elseif row.kind == "header" then
				ChangeList.renderHeader(row, headers)
			else
				ChangeList.renderChange(row, select)
			end
		end)
	end)
end

return PatchVisualizer
end function __DARKLUA_BUNDLE_MODULES.Z():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.Z if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.Z=v end return v.c end end do local function __modImpl()--[[
	Adapter from the diff document's op vocabulary (src/diff_document.rs) to
	the visualizer's Patch. Used by the conflict resolver (one side's impact
	against base/side manifests) and by the diff viewer (the whole document
	against old/new manifests).

	Nodes are materialized from the manifests: an op's instance plus the
	ancestor chain that reaches it. Ops that create instances (add) live in
	the new manifest; everything else addresses the old one, and a reparented
	instance is shown under its new parent, as Rojo places an added node
	under its destination.
]]


local format = __DARKLUA_BUNDLE_MODULES.g()
local types = __DARKLUA_BUNDLE_MODULES.V()
















local function index(manifest: Manifest__DARKLUA_TYPE_aK): Index__DARKLUA_TYPE_aM
	local byId = {}
	for _, node in manifest do
		byId[node.id] = node
	end
	return { byId = byId }
end

local function pathOf(idx: Index__DARKLUA_TYPE_aM, id: number?): string
	local parts = {}
	local cursor = if id then idx.byId[id] else nil
	while cursor do
		table.insert(parts, 1, cursor.name)
		cursor = if cursor.parent then idx.byId[cursor.parent] else nil
	end
	return table.concat(parts, ".")
end

local function fromDocument(options: Options__DARKLUA_TYPE_aL): Patch__DARKLUA_TYPE_ao	
local oldIdx = index(options.old)
	local newIdx = index(options.new)
	local nodes: { [Id__DARKLUA_TYPE_ai]: PatchNodeInput__DARKLUA_TYPE_an} = {}

	-- Materialize a node from a manifest, with its ancestors, preferring the
	-- requested version and falling back to the other for shared ancestors.
	local function ensure(id: number, preferNew: boolean): PatchNodeInput__DARKLUA_TYPE_an?
		local existing = nodes[id]
		if existing then
			return existing
		end
		local first = if preferNew then newIdx else oldIdx
		local second = if preferNew then oldIdx else newIdx
		local record = first.byId[id] or second.byId[id]
		if not record then
			return nil
		end
		local node: PatchNodeInput__DARKLUA_TYPE_an= {
			id = id,
			name = record.name,
			className = record.className,
			parent = record.parent,
			rows = {},
		}
		nodes[id] = node
		if record.parent then
			ensure(record.parent, preferNew)
		end
		return node
	end

	local function rowsOf(node: PatchNodeInput__DARKLUA_TYPE_an): { ChangeRow__DARKLUA_TYPE_al}
		return node.rows :: { ChangeRow__DARKLUA_TYPE_al}
	end

	for _, op in options.ops do
		if op.op == "add" then
			local root = ensure(op.id, true)
			if not root then
				continue
			end
			root.kind = "Add"
			root.subtreeCount = op.instanceCount
			for _, added in op.subtree or {} do
				local node = nodes[added.id]
				if not node then
					node = {
						id = added.id,
						name = added.name,
						className = added.class,
						parent = added.parent,
						rows = {},
					}
					nodes[added.id] = node
					if added.parent then
						ensure(added.parent, true)
					end
				end
				node.kind = "Add"
				if options.addProperties then
					for property, value in added.properties or {} do
						table.insert(rowsOf(node), { property = property, kind = "Add", incoming = value })
					end
				end
			end
		elseif op.op == "remove" then
			local node = ensure(op.id, false)
			if node then
				node.kind = "Remove"
				node.subtreeCount = op.instanceCount
			end
		elseif op.op == "reparent" then
			-- Shown under the destination: the node's parent is the new one.
			local node = ensure(op.id, true)
			if node then
				node.parent = op.to
				if op.to then
					ensure(op.to, true)
				end
				table.insert(rowsOf(node), {
					property = "Parent",
					kind = "Reparent",
					currentText = format.escapeRich(pathOf(oldIdx, op.from)),
					incomingText = format.escapeRich(pathOf(newIdx, op.to)),
				})
			end
		elseif op.op == "setName" then
			local node = ensure(op.id, true)
			if node then
				table.insert(rowsOf(node), {
					property = "Name",
					kind = "Edit",
					currentText = `"{format.escapeRich(tostring(op.before))}"`,
					incomingText = `"{format.escapeRich(tostring(op.after))}"`,
				})
			end
		elseif op.op == "setProperty" then
			local node = ensure(op.id, true)
			if node then
				table.insert(rowsOf(node), {
					property = op.property or "Property",
					kind = "Edit",
					current = op.before,
					incoming = op.after,
				})
			end
		end
	end

	for _, pivot in options.pivots or {} do
		local node = ensure(pivot.id, true)
		if node then
			node.kind = node.kind or "Pivot"
			table.insert(rowsOf(node), {
				property = "Pivot",
				kind = "Pivot",
				currentText = pivot.current,
				incomingText = pivot.incoming,
			})
		end
	end

	return { nodes = nodes }
end

return fromDocument
end function __DARKLUA_BUNDLE_MODULES._():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache._ if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache._=v end return v.c end end do local function __modImpl()--[[
	Direct conflict patch for the selected conflict's staged (or default)
	side: that side's impact ops against the base and side manifests, through
	the shared document adapter and PatchVisualizer. A pivot conflict
	contributes a Pivot row carrying the side's delta.
]]


local Element = __DARKLUA_BUNDLE_MODULES.d()
.miu.Element

local PatchVisualizer = __DARKLUA_BUNDLE_MODULES.Z()
local fromDocument = __DARKLUA_BUNDLE_MODULES._()
local conflicts = __DARKLUA_BUNDLE_MODULES.j()
local cts = __DARKLUA_BUNDLE_MODULES.i()
local format = __DARKLUA_BUNDLE_MODULES.g()
local world = __DARKLUA_BUNDLE_MODULES.h()




local HEADER_HEIGHT = 24

local function resolutionOf(entity: any): string?
	return if world:get(entity, cts.Members)
		then conflicts.groupResolution(entity)
		else world:get(entity, cts.Resolution) :: string?
end

local function buildPatch(entity: any, side: string): (Patch__DARKLUA_TYPE_aD, ImpactSide__DARKLUA_TYPE_aN)
	local impact = conflicts.impact(entity, side)
	local trees = (world:get(cts.VersionTrees, cts.VersionTrees) :: VersionTrees__DARKLUA_TYPE_aP?) or {}

	local pivots = {}
	local info = world:get(entity, if side == "ours" then cts.Ours else cts.Theirs) :: SideInfo__DARKLUA_TYPE_aO?
	if info and info.delta then
		local target = world:get(entity, cts.Target) :: Instance?
		local ids = world:get(cts.VirtualSubjectIds, cts.VirtualSubjectIds) :: { [Instance]: number }?
		local id = if target and ids then ids[target] else nil
		if id then
			table.insert(pivots, {
				id = id,
				current = "base placement",
				incoming = format.escapeRich(format.formatDelta(info.delta)),
			})
		end
	end

	local patch = fromDocument({
		ops = impact.ops,
		old = trees.base or {},
		new = trees[side] or trees.base or {},
		pivots = pivots,
		addProperties = true,
	})
	return patch, impact
end

local function ImpactPanel()
	local entity = world:get(cts.SelectedConflict, cts.SelectedConflict)
	local resolution = entity and resolutionOf(entity) or nil
	local side = if resolution == "theirs" then "theirs" else "ours"

	local patch: Patch__DARKLUA_TYPE_aD= { nodes = {} }
	local impact: ImpactSide__DARKLUA_TYPE_aN? = nil
	if entity then
		patch, impact = buildPatch(entity, side)
	end

	Element("Frame", {
		id = "impact",
		Size = UDim2.fromScale(1, 1),
		tags = { "surface", "vs-border" },
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = if impact
				then `DIRECT DIFF — {impact.instanceCount} CHANGED · {impact.propertyCount} {if impact.propertyCount
						== 1
					then "PROPERTY"
					else "PROPERTIES"}`
				else "DIRECT DIFF",
			Size = UDim2.new(1, 0, 0, HEADER_HEIGHT),
			tags = { "header-band" },
		})

		PatchVisualizer({
			patch = patch,
			stateKey = if entity then `{tostring(entity)}:{side}` else nil,
			headers = { "Property", "Base", string.upper(side:sub(1, 1)) .. side:sub(2) },
			selectedId = world:get(cts.SelectedExplorerId, cts.SelectedExplorerId) :: number?,
			onSelect = function(node)
				conflicts.selectExplorerId(node.id :: number)
			end,
			emptyText = "No direct changes on this side",
			Size = UDim2.new(1, 0, 1, -HEADER_HEIGHT),
			Position = UDim2.fromOffset(0, HEADER_HEIGHT),
		})
	end)
end

return ImpactPanel
end function __DARKLUA_BUNDLE_MODULES.aa():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.aa if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.aa=v end return v.c end end do local function __modImpl()-- Direct conflict diff dock widget, synchronized through SelectedConflict.


local packages = __DARKLUA_BUNDLE_MODULES.d()
local Element = packages.miu.Element
local Portal = packages.miu.Portal

local theme = __DARKLUA_BUNDLE_MODULES.q()

local types = __DARKLUA_BUNDLE_MODULES.m()
local widgets = __DARKLUA_BUNDLE_MODULES.x()

local ImpactPanel = __DARKLUA_BUNDLE_MODULES.aa()

return function(_world: World__DARKLUA_TYPE_P, _cts: Cts__DARKLUA_TYPE_Q, scheduler: Scheduler__DARKLUA_TYPE_R)
	local widget = widgets.create("GitRbxMergeDirectDiff", "git-rbx — direct diff", Enum.InitialDockState.Right, {
		width = 500,
		height = 360,
		minWidth = 320,
		minHeight = 180,
	})

	scheduler:addSystem({
		name = "ui_diff",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			debug.profilebegin("gitrbx_ui_diff")
			Portal(widget, "git-rbx-direct-diff", function()
				theme.link()
				Element("Frame", {
					id = "diff-widget",
					Size = UDim2.fromScale(1, 1),
					tags = { "surface", "vs-border" },
				}, function()
					ImpactPanel()
				end)
			end)
			debug.profileend()
		end,
	})
end
end function __DARKLUA_BUNDLE_MODULES.ab():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.ab if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.ab=v end return v.c end end do local function __modImpl()--[[
	End-of-frame cleanup sweep, after game-prototype's core/entitiesDestroy:
	anything tagged Destroy is deleted in one batch. Registered last so
	requests consumed earlier in the frame don't live past it.
]]


local packages = __DARKLUA_BUNDLE_MODULES.d()

local types = __DARKLUA_BUNDLE_MODULES.m()

return function(world: World__DARKLUA_TYPE_P, cts: Cts__DARKLUA_TYPE_Q, scheduler: Scheduler__DARKLUA_TYPE_R)
	scheduler:addSystem({
		name = "entities_destroy",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			for entity in world:each(cts.Destroy) do
				world:delete(entity)
			end
		end,
	})
end
end function __DARKLUA_BUNDLE_MODULES.ac():typeof(__modImpl())local v=__DARKLUA_BUNDLE_MODULES.cache.ac if not v then v={c=__modImpl()}__DARKLUA_BUNDLE_MODULES.cache.ac=v end return v.c end end end--[[
	git-rbx studio resolver — visual front end over the in-file conflict state.

	Run against the conflicted file (places open directly; models import
	into a preview folder). `git rbx resolve <file> --studio` launches this
	tree straight from the checkout; by hand:

	  rodeo run --place ./merged.rbxl --focus src/init.luau -- ./merged.rbxl
	  rodeo run --place --focus src/init.luau -- ./merged.rbxm

	The run owns the Studio it spawns, so Studio closes itself when the
	session completes — pass --detach to keep it open.

	Discovery is entirely in-DOM (the __GitRbxConflicts container + GitRbxConflict
	tags that `git rbx merge` stamped). Decisions are STAGED in the session —
	nothing touches the disk file until Complete, which batches the canonical
	CLI calls host-side via @rodeo/process:

	  git rbx resolve <file> --take <side> --entry <Conflict_N>   (per decision)
	  git rbx resolve <file> --finalize

	The Studio DOM is a COPY of the file — the disk file stays authoritative and
	only git-rbx writes it, only at Complete. Abandoning the session leaves the
	conflicted file exactly as `git rbx merge` wrote it.

	Layout: this file is the composition root — parse args, read the stamped
	container into conflict entities + singletons, register systems. The world
	is the source of truth; all behavior lives in ecs/systems/ as scheduled
	loops (each returns a register function taking world/cts/scheduler), with
	stateless helpers in ecs/systems/library/. library/ holds the non-ecs
	support modules (config, format, theme, transport, viewport math);
	ui/ holds the miu panels the ui systems render (ExplorerPanel/,
	PropertyPanel/, and the conflicts-panel pieces).

	Args: <merged-file> [--git-rbx <path-to-binary>] [--auto <side>]
]]


local process = require("@rodeo/process")
local HttpService = game:GetService("HttpService")

local argparse = __DARKLUA_BUNDLE_MODULES.b()

-- Real packages, not vendors: packages.luau rojo-builds roblox_packages and
-- imports it, so the pesde shims resolve as instances.
local packages = __DARKLUA_BUNDLE_MODULES.d()
local jecs = packages.jecs

local config = __DARKLUA_BUNDLE_MODULES.e()
local stateContainer = __DARKLUA_BUNDLE_MODULES.f()
local transport = __DARKLUA_BUNDLE_MODULES.k()

local cts = __DARKLUA_BUNDLE_MODULES.i()
local scheduler = __DARKLUA_BUNDLE_MODULES.l()
local world = __DARKLUA_BUNDLE_MODULES.h()

local CONTAINER_NAME = "__GitRbxConflicts"

local parser = argparse("git-rbx-resolver", "Resolve git-rbx merge conflicts visually in Studio")
parser:argument("file", "Conflicted merged file written by `git rbx merge`")
parser:option("--git-rbx", "Path to the git-rbx binary"):target("gitRbxBin"):default("git-rbx")
parser:option("--auto", "Debug: stage every conflict to this side and Complete immediately"):target("autoSide")

local args = parser:parse(process.args)
local mergedFile = args.file

-- ============================================================================
-- Boot: read the stamped container once into conflict entities + singletons —
-- from here on the world is the source of truth for merge + session state.
-- ============================================================================

local container = stateContainer.find(mergedFile, CONTAINER_NAME)

if not container then
	warn(`no {CONTAINER_NAME} container in this place — nothing to resolve`)
	return "no-conflicts"
end

type ConflictRecord = {
	name: string,
	kind: string,
	path: string,
	property: string?,
	properties: { string }?,
	resolved: string?,
	group: string?,
	target: Instance,
	entry: Instance,
	pivotOrder: number?,
	pivotParentOrder: number?,
	impactOurs: ImpactSide__DARKLUA_TYPE_F?,
	impactTheirs: ImpactSide__DARKLUA_TYPE_F?,
}

type GroupRecord = {
	name: string,
	kind: string,
	path: string,
	target: Instance?,
	entry: Instance,
	deltaOurs: CFrame?,
	deltaTheirs: CFrame?,
}

local records: { ConflictRecord } = {}
local groupRecords: { GroupRecord } = {}

local function readImpact(entry: Instance, side: string): ImpactSide__DARKLUA_TYPE_F?
	local sideFolder = entry:FindFirstChild(side)
	local value = sideFolder and sideFolder:FindFirstChild("__GitRbxImpact")
	if not value or not value:IsA("StringValue") then
		return nil
	end
	local ok, decoded = pcall(HttpService.JSONDecode, HttpService, value.Value)
	if not ok then
		warn(`invalid impact metadata on {entry.Name}.{side}: {decoded}`)
		return nil
	end
	return decoded :: ImpactSide__DARKLUA_TYPE_F
end

for _, entry in container:GetChildren() do
	local kind = entry:GetAttribute("Kind") :: string?
	local groupKind = entry:GetAttribute("GroupKind") :: string?
	if kind then
		table.insert(records, {
			name = entry.Name,
			kind = kind,
			path = entry:GetAttribute("Path") :: string,
			property = entry:GetAttribute("Property") :: string?,
			properties = (function(): { string }?
				local encoded = entry:GetAttribute("Properties") :: string?
				return if encoded then string.split(encoded, ",") else nil
			end)(),
			resolved = entry:GetAttribute("Resolved") :: string?,
			group = entry:GetAttribute("Group") :: string?,
			target = (entry:FindFirstChild("Target") :: ObjectValue).Value :: Instance,
			entry = entry,
			pivotOrder = entry:GetAttribute("PivotOrder") :: number?,
			pivotParentOrder = entry:GetAttribute("PivotParentOrder") :: number?,
			impactOurs = readImpact(entry, "Ours"),
			impactTheirs = readImpact(entry, "Theirs"),
		})
	elseif groupKind then
		local targetValue = entry:FindFirstChild("Target") :: ObjectValue?
		table.insert(groupRecords, {
			name = entry.Name,
			kind = groupKind,
			path = entry:GetAttribute("Path") :: string,
			target = if targetValue then targetValue.Value else nil,
			entry = entry,
			deltaOurs = entry:GetAttribute("DeltaOurs") :: CFrame?,
			deltaTheirs = entry:GetAttribute("DeltaTheirs") :: CFrame?,
		})
	end
end
table.sort(records, function(a, b)
	return a.name < b.name
end)

config.mergedFile = mergedFile
config.gitRbxBin = args.gitRbxBin

local conflictEntities: { any } = {}
local membersByGroup: { [string]: { any } } = {}
for index, record in records do
	local entity = world:entity()
	world:set(entity, jecs.Name, record.name)
	world:set(entity, cts.Order, index)
	world:set(entity, cts.Kind, record.kind)
	world:set(entity, cts.Path, record.path)
	world:set(entity, cts.Target, record.target)
	world:set(entity, cts.Entry, record.entry)
	if record.pivotOrder then
		world:set(entity, cts.PivotOrder, record.pivotOrder)
	end
	if record.pivotParentOrder then
		world:set(entity, cts.PivotParentOrder, record.pivotParentOrder)
	end
	if record.property then
		world:set(entity, cts.Property, record.property)
	end
	if record.properties then
		world:set(entity, cts.Properties, record.properties)
	end
	if record.resolved then
		world:set(entity, cts.Resolution, record.resolved)
	end
	if record.impactOurs then
		world:set(entity, cts.ImpactOurs, record.impactOurs)
	end
	if record.impactTheirs then
		world:set(entity, cts.ImpactTheirs, record.impactTheirs)
	end
	if record.group then
		world:set(entity, cts.Group, record.group)
		local members = membersByGroup[record.group] or {}
		membersByGroup[record.group] = members
		table.insert(members, entity)
	end
	table.insert(conflictEntities, entity)
end
world:set(cts.Conflicts, cts.Conflicts, conflictEntities)
world:set(cts.Container, cts.Container, container)

local pivotPlan: { PivotApplication__DARKLUA_TYPE_L} = {}
local pivotPlanFolder = container:FindFirstChild("PivotPlan")
if pivotPlanFolder then
	for _, entry in pivotPlanFolder:GetChildren() do
		local targetValue = entry:FindFirstChild("Target")
		local target = if targetValue and targetValue:IsA("ObjectValue") then targetValue.Value else nil
		local order = entry:GetAttribute("PivotOrder") :: number?
		local parentOrder = entry:GetAttribute("PivotParentOrder") :: number?
		local delta = entry:GetAttribute("Delta") :: CFrame?
		if target and order and delta then
			table.insert(pivotPlan, {
				target = target,
				order = order,
				parentOrder = parentOrder,
				delta = delta,
			})
		end
	end
	table.sort(pivotPlan, function(a, b)
		return a.order < b.order
	end)
end
world:set(cts.PivotPlan, cts.PivotPlan, pivotPlan)

-- Rust captures these complete hierarchies before mutating base into the
-- partial merge. They are compact data, not cloned Roblox instances; logical
-- ids shared across versions map back to live result subjects when available.
local virtualTreesFolder = container:FindFirstChild("VirtualTrees")
local virtualSubjectIds: { [Instance]: number } = {}
if virtualTreesFolder then
	local versionTrees: VersionTrees__DARKLUA_TYPE_K= {}
	for _, side in { "Base", "Ours", "Theirs" } do
		local treeFolder = virtualTreesFolder:FindFirstChild(side)
		if not treeFolder then
			continue
		end
		local chunks = treeFolder:GetChildren()
		table.sort(chunks, function(a, b)
			return a.Name < b.Name
		end)
		local encoded = ""
		for _, chunk in chunks do
			if chunk:IsA("StringValue") then
				encoded ..= chunk.Value
			end
		end
		local decoded = HttpService:JSONDecode(encoded)
		local nodes: { VersionTreeNode__DARKLUA_TYPE_J} = table.create(#decoded)
		for index, record in decoded do
			nodes[index] = {
				id = record[1],
				parent = if record[2] == 0 then nil else record[2],
				name = record[3],
				className = record[4],
			}
		end
		versionTrees[string.lower(side)] = nodes
	end
	world:set(cts.VersionTrees, cts.VersionTrees, versionTrees)

	local subjects: { [number]: Instance } = {}
	local subjectsFolder = virtualTreesFolder:FindFirstChild("Subjects")
	if subjectsFolder then
		for _, value in subjectsFolder:GetChildren() do
			if value:IsA("ObjectValue") and value.Value then
				local id = tonumber(string.sub(value.Name, 2))
				if id then
					subjects[id] = value.Value
					virtualSubjectIds[value.Value] = id
				end
			end
		end
	end
	world:set(cts.VirtualSubjects, cts.VirtualSubjects, subjects)
end
world:set(cts.VirtualSubjectIds, cts.VirtualSubjectIds, virtualSubjectIds)

-- Rigid-group entities: metadata over their member conflicts. The Cards
-- singleton is the display list — each group appears once, at its first
-- member's position; grouped members stay out of it.
local groupEntities: { [string]: any } = {}
for _, groupRecord in groupRecords do
	local entity = world:entity()
	world:set(entity, jecs.Name, groupRecord.name)
	world:set(entity, cts.Kind, groupRecord.kind)
	world:set(entity, cts.Path, groupRecord.path)
	world:set(entity, cts.Entry, groupRecord.entry)
	world:set(entity, cts.Members, membersByGroup[groupRecord.name] or {})
	if groupRecord.target then
		world:set(entity, cts.Target, groupRecord.target)
	end
	if groupRecord.deltaOurs then
		world:set(entity, cts.DeltaOurs, groupRecord.deltaOurs)
	end
	if groupRecord.deltaTheirs then
		world:set(entity, cts.DeltaTheirs, groupRecord.deltaTheirs)
	end
	groupEntities[groupRecord.name] = entity
end

local cards: { any } = {}
local emitted: { [string]: boolean } = {}
for index, record in records do
	local group = record.group
	if group and groupEntities[group] then
		if not emitted[group] then
			emitted[group] = true
			table.insert(cards, groupEntities[group])
		end
	else
		table.insert(cards, conflictEntities[index])
	end
end
world:set(cts.Cards, cts.Cards, cards)

world:set(cts.ViewTab, cts.ViewTab, "result")
local firstConflict = cards[1]
if firstConflict then
	world:set(cts.SelectedConflict, cts.SelectedConflict, firstConflict)
	local target = world:get(firstConflict, cts.Target) :: Instance?
	if target then
		world:set(cts.SelectedInstance, cts.SelectedInstance, target)
		local virtualId = virtualSubjectIds[target]
		if virtualId then
			world:set(cts.SelectedExplorerId, cts.SelectedExplorerId, virtualId)
		end
	end
	world:set(cts.ExplorerRevealGeneration, cts.ExplorerRevealGeneration, 1)
end

print(`git-rbx resolver: {#records} conflict(s) in {mergedFile}`)

-- ============================================================================
-- Systems; the run stays alive until finalized
-- ============================================================================

-- Same-phase run order = registration order: previews render, the mirror
-- diffs the DOM, Explorer views project it, highlights re-adorn, focus
-- requests resolve, UI last.
for _, registerSystem in
	{
__DARKLUA_BUNDLE_MODULES.n(),
__DARKLUA_BUNDLE_MODULES.o(),
__DARKLUA_BUNDLE_MODULES.p(),
__DARKLUA_BUNDLE_MODULES.r(),
__DARKLUA_BUNDLE_MODULES.w(),
__DARKLUA_BUNDLE_MODULES.E(),
__DARKLUA_BUNDLE_MODULES.N(),
__DARKLUA_BUNDLE_MODULES.T(),
__DARKLUA_BUNDLE_MODULES.ab(),
__DARKLUA_BUNDLE_MODULES.ac(),
	}
do
	registerSystem(world, cts, scheduler)
end
-- Debug drive-through: stage everything to one side and Complete, so the
-- whole pipeline can run headlessly (rodeo run ... -- <file> --auto ours)
if args.autoSide then
	for _, entity in conflictEntities do
		world:set(entity, cts.Resolution, args.autoSide)
	end
	transport.complete()
end

while not world:has(cts.Finished, cts.Finished) do
	task.wait(0.25)
end

return `finalized {#records} conflict(s) — {mergedFile} is clean`
