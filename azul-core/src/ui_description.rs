use azul_css::{Css, CssDeclaration, CssProperty, CssPropertyType};
use std::{collections::BTreeMap, fmt};
use {
    callbacks::{FocusTarget, HitTestItem},
    dom::{DomId, DomString, NodeData},
    id_tree::{Arena, NodeDataContainer, NodeId},
    ui_state::{HoverGroup, UiState},
    FastHashMap,
};

pub struct UiDescription<T> {
    /// DOM ID of this arena (so that multiple DOMs / IFrames can be displayed in one window)
    pub dom_id: DomId,
    /// The DOM data (arena-allocated)
    pub ui_descr_arena: Arena<NodeData<T>>,
    /// ID of the root node of the arena (usually NodeId(0))
    pub ui_descr_root: NodeId,
    /// This field is created from the Css
    pub styled_nodes: NodeDataContainer<StyledNode>,
    /// The style properties that should be overridden for this frame, cloned from the `Css`
    pub dynamic_css_overrides: BTreeMap<NodeId, FastHashMap<DomString, CssProperty>>,
    /// In order to hit-test :hover and :active selectors, need to insert tags for all rectangles
    /// that have a non-:hover path, for example if we have `#thing:hover`, then all nodes selected by `#thing`
    /// need to get a TagId, otherwise, they can't be hit-tested.
    pub selected_hover_nodes: BTreeMap<NodeId, HoverGroup>,
}

impl<T> fmt::Debug for UiDescription<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "UiDescription {{ \
            dom_id: {:?},
            ui_descr_arena: {:?},
            ui_descr_root: {:?},
            styled_nodes: {:?},
            dynamic_css_overrides: {:?},
            selected_hover_nodes: {:?},
        }}",
            self.dom_id,
            self.ui_descr_arena,
            self.ui_descr_root,
            self.styled_nodes,
            self.dynamic_css_overrides,
            self.selected_hover_nodes,
        )
    }
}

impl<T> Clone for UiDescription<T> {
    fn clone(&self) -> Self {
        Self {
            dom_id: self.dom_id.clone(),
            ui_descr_arena: self.ui_descr_arena.clone(),
            ui_descr_root: self.ui_descr_root,
            styled_nodes: self.styled_nodes.clone(),
            dynamic_css_overrides: self.dynamic_css_overrides.clone(),
            selected_hover_nodes: self.selected_hover_nodes.clone(),
        }
    }
}

impl<T> Default for UiDescription<T> {
    fn default() -> Self {
        use {dom::Dom, ui_state::ui_state_from_dom};

        let default_dom = Dom::div();
        let hovered_nodes = BTreeMap::new();
        let is_mouse_down = false;

        let mut focused_node = None;
        let mut focus_target = None;
        let mut ui_state = ui_state_from_dom(default_dom, None);

        Self::match_css_to_dom(
            &mut ui_state,
            &Css::default(),
            &mut focused_node,
            &mut focus_target,
            &hovered_nodes,
            is_mouse_down,
        )
    }
}

impl<T> UiDescription<T> {
    /// Applies the styles to the nodes calculated from the `layout_screen`
    /// function and calculates the final display list that is submitted to the
    /// renderer.
    pub fn match_css_to_dom(
        ui_state: &mut UiState<T>,
        style: &Css,
        focused_node: &mut Option<(DomId, NodeId)>,
        pending_focus_target: &mut Option<FocusTarget>,
        hovered_nodes: &BTreeMap<NodeId, HitTestItem>,
        is_mouse_down: bool,
    ) -> Self {
        use ui_state::ui_state_create_tags_for_hover_nodes;

        let ui_description = ::style::match_dom_selectors(
            ui_state,
            &style,
            focused_node,
            pending_focus_target,
            hovered_nodes,
            is_mouse_down,
        );

        // Important: Create all the tags for the :hover and :active selectors
        ui_state_create_tags_for_hover_nodes(ui_state, &ui_description.selected_hover_nodes);

        ui_description
    }
}

#[derive(Debug, Default, Clone, PartialEq, Hash, PartialOrd, Eq, Ord)]
pub struct StyledNode {
    /// The CSS constraints, after the cascading step
    pub css_constraints: BTreeMap<CssPropertyType, CssDeclaration>,
}
