# Roblox Studio UI Analysis

Analysis of the RobloxStudio binary to understand how Explorer and Properties panels are implemented for visual fidelity in rbx-diff-viewer.

## Key Findings

### Component Architecture

Studio uses Qt (specifically QTreeWidget, not QAbstractItemModel) with custom wrappers:

**Explorer Tree:**
- `RBX::Studio::ExplorerTreeWidget` - Main explorer widget
- `RBX::Studio::ExplorerTreeWidgetItem` - Individual tree items
- `RBX::Studio::DiffExplorerTreeWidget` - Specialized for diff views (relevant!)

**Properties Panel:**
- `PropertiesWidget::RobloxPropertiesWidget` - Main properties container
- `PropertiesWidget::RobloxPropertiesTreeWidget` - Properties tree (wraps QTreeWidget)

**Qt Wrappers:**
- `RbxQt::StyledObject<T, QWidget>` - Templated wrapper for theming support
- Example: `RbxQt::StyledObject<RobloxPropertiesTreeWidget, QTreeWidget>`

### Property Item Types

Studio has dedicated property item classes for each type:

| Class | Roblox Type | Notes |
|-------|-------------|-------|
| `BoolPropertyItem` | bool | Checkbox |
| `IntPropertyItem` | int | Integer input |
| `DoublePropertyItem` | number | Float input |
| `StringPropertyItem` | string | Text input |
| `EnumPropertyItem` | Enum | Dropdown |
| `ColorPropertyItem` | Color3 | Color picker |
| `BrickColorPropertyItem` | BrickColor | BrickColor picker |
| `ColorSequencePropertyItem` | ColorSequence | Gradient editor |
| `NumberSequencePropertyItem` | NumberSequence | Graph editor |
| `NumberRangePropertyItem` | NumberRange | Min/Max range |
| `Vector2PropertyItem` | Vector2 | X, Y fields |
| `VectorPropertyItem` | Vector3 | X, Y, Z fields |
| `CFramePropertyItem` | CFrame | Position + rotation |
| `UDim2PropertyItem` | UDim2 | Scale/Offset pairs |
| `RectPropertyItem` | Rect | Min/Max points |
| `InstancePropertyItem` | Instance ref | Object picker |
| `ContentIdPropertyItem` | ContentId | Asset ID input |
| `AnimationIdPropertyItem` | AnimationId | Animation asset |
| `SoundIdPropertyItem` | SoundId | Sound asset |
| `MeshIdPropertyItem` | MeshId | Mesh asset |
| `VideoIdPropertyItem` | VideoId | Video asset |

Special sections:
- `CategoryItem` - Property category header
- `AttributesSection` - Custom attributes
- `TagsSection` - CollectionService tags
- `BehaviorBindingsSection` - Script bindings

### Theme System

Studio uses `RBX::Studio::WidgetThemeManager` with property-based colors:

**Settings namespace:** `Studio.App.Property.*`

**Background Colors:**
- `BackgroundColor` - Main background
- `SelectionBackgroundColor` - Selected item background
- `SelectionColor` - Selection highlight
- `FindSelectionBackgroundColor` - Search result highlight
- `MatchingWordBackgroundColor` - Word match highlight
- `MenuItemBackgroundColor` - Menu item background
- `SelectedMenuItemBackgroundColor` - Selected menu item
- `DocViewCodeBackgroundColor` - Code viewer background
- `ScriptEditorScrollbarBackgroundColor` - Scrollbar background

**Text Colors:**
- `TextColor` - Primary text
- `PrimaryTextColor` - Primary text (alternate)
- `SelectedTextColor` - Selected text color

**UI State Colors:**
- `ActiveColor` - Active state
- `ActiveHoverOverColor` - Active + hover
- `HoverOverColor` - Hover state
- `SelectColor` - Selection
- `SelectHoverColor` - Selection + hover

### Qt Stylesheet Patterns

Studio uses Qt stylesheets with format string placeholders (%1, %2, etc.):

```css
/* Tree view item styling */
QTreeView#ObjectNamePlaceholder::branch { ... }
QTreeView#ObjectNamePlaceholder::item { ... }
QTreeView#ObjectNamePlaceholder::item:hover,
QTreeView#ObjectNamePlaceholder::branch:hover { ... }
QTreeView#ObjectNamePlaceholder::item:selected:!active,
QTreeView#ObjectNamePlaceholder::item:selected:active,
QTreeView#ObjectNamePlaceholder::branch:selected:!active,
QTreeView#ObjectNamePlaceholder::branch:selected:active { ... }

/* Disable hover highlight in tree widget */
QTreeWidget::item:hover { background-color: none; }
```

**Common CSS Properties Used:**
```css
background-color: %1;           /* Theme color placeholder */
alternate-background-color: %6;
color: %3;
border: 1px solid %1;
border-radius: 4px | 6px | 8px;
padding: 4px 8px;
margin-left: 10px;
font-size: 14px | 16px | 18px;
selection-background-color: %4;
```

### Foundation Dark Theme Colors

Extracted from embedded theme resources (`FoundationDarkTheme.json`):

**Background Colors (Dark):**
```css
rgba(18, 18, 21, 1)    /* #121215 - Darkest background */
rgba(25, 26, 31, 1)    /* #191a1f - Dark background */
rgba(32, 34, 39, 1)    /* #202227 - Slightly lighter dark */
```

**Overlay Colors (for hover/states):**
```css
rgba(208, 217, 251, 0.06)  /* Light overlay 6% - subtle hover */
rgba(208, 217, 251, 0.12)  /* Light overlay 12% - active */
rgba(208, 217, 251, 0.16)  /* Light overlay 16% - pressed */
```

**Light Theme Background:**
```css
rgba(247, 247, 248, 1)     /* #f7f7f8 - Light background */
rgba(235, 241, 255, 1)     /* #ebf1ff - Light alt background */
```

**Accent Colors:**
```css
rgb(246, 136, 2)    /* #f68802 - Orange (brand accent) */
rgb(247, 75, 82)    /* #f74b52 - Red (error/danger) */
rgb(53, 181, 255)   /* #35b5ff - Blue (links/info) */
rgb(253, 251, 172)  /* #fdfbac - Yellow (warning) */
rgb(2, 183, 87)     /* #02b757 - Green (success) */
```

**Text Colors:**
```css
rgb(255, 255, 255)  /* White - primary text on dark */
rgb(102, 102, 102)  /* #666666 - secondary/muted text */
```

**Shadow/Overlay:**
```css
rgba(0, 20, 92, 0.5)    /* Dark blue shadow */
rgba(18, 24, 50, 0.19)  /* Subtle shadow */
rgba(27, 37, 75, 0.06)  /* Light blue tint */
rgba(27, 37, 75, 0.12)  /* Light blue tint (stronger) */
rgba(27, 37, 75, 0.16)  /* Light blue tint (strongest) */
```

### Hex Color Values Found

Additional hardcoded colors:
```css
#474747  /* Dark gray - borders */
#313131  /* Darker gray - backgrounds */
#333333  /* Dark gray - alternate */
#ffffff  /* White */
#000000  /* Black */
#d86868  /* Soft red */
#bbbbbb  /* Light gray */
#aaaaaa  /* Medium gray */
```

### Explorer Settings (Feature Flags)

Explorer behavior is controlled by these settings:
- `ExplorerFilter` - Search/filter
- `ExplorerDragAndDrop` - Drag and drop support
- `ExplorerExpandAll` / `ExplorerCollapseAll` - Expand/collapse
- `ExplorerEnhanced` - Enhanced explorer mode
- `ExplorerImageIndex` - Icon indices
- `ExplorerOrder` - Sort order
- `ExplorerPackageIconColumn` - Package status icons

### Explorer Tree Widget Methods

Key methods found in `ExplorerTreeWidget`:
- `expandAllInstances()` - Expand all nodes
- `collapseAllInstances()` - Collapse all nodes
- `syncInstanceSelection()` - Sync with Selection service
- `setSelectionServiceProvider()` - Connect to DataModel
- `initTreeViewWithDMRoot()` - Initialize with DataModel
- `onPackageChanged()` - Handle package updates

Key methods in `ExplorerTreeWidgetItem`:
- `updateIcon()` - Update item icon
- `updateQtData()` - Update when instance changes
- `updateHiddenBySettingsAndRerender()` - Update visibility

### Data Structures

**Instance representation in tree:**
```cpp
// Conceptual structure based on method signatures
class ExplorerTreeWidgetItem : public QTreeWidgetItem {
    shared_ptr<Instance> instance;

    void updateIcon();
    void updateQtData(shared_ptr<Instance>);
    void updateHiddenBySettingsAndRerender();
};
```

**Property representation:**
```cpp
// Each property type has a dedicated item class
class PropertyItem {
    QString name;
    QString typeName;
    Variant value;
    bool isReadOnly;
};

// Category groups properties
class CategoryItem {
    QString categoryName;
    vector<PropertyItem*> properties;
};
```

## Recommendations for rbx-diff-viewer

### 1. Tree Structure
Use a similar tree item approach:
- Each tree item holds an instance reference
- Update icons and names from instance properties
- Support expand/collapse state

### 2. Property Panel
Implement dedicated formatters for each property type:
- Use type-specific display (checkboxes for bool, color swatches for Color3)
- Group by category (Appearance, Data, etc.)
- Sort alphabetically within categories

### 3. Styling
Apply Studio Foundation Dark Theme colors:
```css
:root {
  /* Foundation Dark Theme - extracted from Studio binary */
  --bg-darkest: rgba(18, 18, 21, 1);      /* #121215 */
  --bg-dark: rgba(25, 26, 31, 1);          /* #191a1f */
  --bg-light: rgba(32, 34, 39, 1);         /* #202227 */

  /* Overlay colors for states */
  --overlay-hover: rgba(208, 217, 251, 0.06);
  --overlay-active: rgba(208, 217, 251, 0.12);
  --overlay-pressed: rgba(208, 217, 251, 0.16);

  /* Accent colors */
  --accent-orange: rgb(246, 136, 2);       /* Brand */
  --accent-blue: rgb(53, 181, 255);        /* Links */
  --accent-red: rgb(247, 75, 82);          /* Error */
  --accent-green: rgb(2, 183, 87);         /* Success */
  --accent-yellow: rgb(253, 251, 172);     /* Warning */

  /* Text */
  --text-primary: rgb(255, 255, 255);
  --text-secondary: rgb(102, 102, 102);

  /* Borders */
  --border: #474747;
  --border-dark: #313131;
}

/* Tree items */
.tree-item {
  padding: 2px 4px;
  background: transparent;
}
.tree-item:hover {
  background: var(--overlay-hover);
}
.tree-item.selected {
  background: var(--overlay-active);
}

/* Properties */
.property-row {
  display: flex;
  padding: 2px 8px;
}
.property-name {
  color: var(--text-primary);
  width: 120px;
}
.property-value {
  color: var(--text-secondary);
}
```

### 4. Icons
- Use `StudioService:GetClassIcon()` for class icons
- Icons are stored as sprite sheets with ImageRectOffset
- We already extracted these to `images/` directory

## Files Referenced

- Binary: `/Applications/RobloxStudio.app/Contents/MacOS/RobloxStudio` (164MB, arm64 Mach-O)
- Settings: `~/Library/Preferences/com.roblox.RobloxStudio.plist`
- Class icons: Generated via `generate_class_icons.luau`

## Analysis Methods Used

1. `strings` - Extract readable strings
2. `nm -C` - Extract demangled symbols
3. Ghidra - Deep binary analysis (pending)
