use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    window::PrimaryWindow,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_lunex::prelude::*;

#[derive(Resource, Default, Clone, Copy, Debug)]
struct SelectionBox(Option<(Vec2, Vec2)>);

impl SelectionBox {
    fn within_selection(&self, pos: Vec2) -> bool {
        let Some((to, from)) = self.0 else {
            return false;
        };

        // NOTE: can be either < or <=
        let within_x = to.x < pos.x && pos.x < from.x;
        let within_y = to.y > pos.y && pos.y > from.y;

        within_x && within_y
    }
}

#[derive(Event, Debug)]
struct FinishedSelectingEvent(SelectionBox);

#[derive(Event, Debug)]
struct DeselectEvent;

#[derive(Event)]
struct MoveToEvent(MoveTo);

#[derive(Component)]
struct SelectionDisplay;

#[derive(Component)]
struct Selected;

#[derive(Component)]
struct Selectable;

#[derive(Component)]
struct Border;

#[derive(Component)]
struct MoveTo(Vec2);

#[derive(Component)]
struct TroopVelocity(f32);

const CIRCLE_RADIUS: f32 = 10.0;
const BORDER_OFFSET: f32 = 1.;
const DISTANCE_TOLERANCE: f32 = 1. / (1 >> 8) as f32;

fn setup(
    mut cmd: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    cmd.spawn(Camera2dBundle::default());
    let circle_mesh = Mesh2dHandle(meshes.add(Circle {
        radius: CIRCLE_RADIUS,
    }));
    let rectangle_mesh = Mesh2dHandle(meshes.add(Rectangle {
        half_size: Vec2::new(0.5, 0.5),
    }));

    cmd.spawn(MaterialMesh2dBundle {
        mesh: circle_mesh,
        material: materials.add(Color::linear_rgba(1.0, 0.0, 0.0, 1.0)),
        ..default()
    })
    .insert(Selectable)
    .insert(TroopVelocity(1.0));

    cmd.spawn(MaterialMesh2dBundle {
        mesh: rectangle_mesh,
        material: materials.add(Color::linear_rgba(0.0, 0.0, 1.0, 0.5)),
        visibility: Visibility::Hidden,
        ..default()
    })
    .insert(SelectionDisplay);

    cmd.spawn((
        UiTreeBundle::<MainUi> {
            tree: UiTree::new2d("MainUiSystem"),
            ..default()
        },
        SourceFromCamera,
    ))
    .with_children(|ui| {
        ui.spawn((
            UiLink::<MainUi>::path("Root"),
            UiLayout::boundary()
                .pos1(Ab(20.0))
                .pos2(Rl(100.0) - Ab(20.0))
                .pack::<Base>(),
        ));
    });
}

fn handle_mouse_input(
    windows_query: Query<&Window, With<PrimaryWindow>>,
    mut selection_box: ResMut<SelectionBox>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut finished_selecting_event_writer: EventWriter<FinishedSelectingEvent>,
    mut deselect_event_writer: EventWriter<DeselectEvent>,
) {
    let Some(window_position) = windows_query.single().cursor_position() else {
        return;
    };
    let world_position = window_position - windows_query.single().size() / 2.;
    let world_position = Vec2::new(world_position.x, -world_position.y);

    if buttons.just_pressed(MouseButton::Left) {
        selection_box.0 = Some((world_position, world_position));
    }

    if buttons.pressed(MouseButton::Left) {
        if let Some(selection) = selection_box.0 {
            selection_box.0 = Some((selection.0, world_position));
        }
    }

    if buttons.just_released(MouseButton::Left) {
        finished_selecting_event_writer.send(FinishedSelectingEvent(*selection_box));
        *selection_box = SelectionBox(None);
    }

    if buttons.just_pressed(MouseButton::Right) {
        deselect_event_writer.send(DeselectEvent);
        *selection_box = SelectionBox(None);
    }
}

fn select_entities(
    mut finished_selecting_event_reader: EventReader<FinishedSelectingEvent>,
    entities_query: Query<(&Transform, Entity), With<Selectable>>,
    mut cmd: Commands,
) {
    let Some(finished_selection_box) = finished_selecting_event_reader.read().next() else {
        return;
    };

    for (transform, entity) in &entities_query {
        let is_inside_selection = finished_selection_box
            .0
            .within_selection(transform.translation.xy());
        if !is_inside_selection {
            continue;
        }

        cmd.entity(entity).insert((Selected, Border));
    }
}

fn display_selection_box(
    mut selection_box_display_query: Query<
        (&mut Visibility, &mut Transform),
        With<SelectionDisplay>,
    >,
    selection_box: Res<SelectionBox>,
) {
    let Some((mut visibility, mut selection_box_display_transform)) =
        selection_box_display_query.iter_mut().next()
    else {
        return;
    };

    *visibility = selection_box
        .0
        .map(|_| Visibility::Visible)
        .unwrap_or(Visibility::Hidden);

    let Some((from, to)) = selection_box.0 else {
        return;
    };

    let center = (from + to) / 2.;
    let scale = to - from;

    selection_box_display_transform.scale = Vec3::new(scale.x, scale.y, 0.);
    selection_box_display_transform.translation = Vec3::new(center.x, center.y, 0.)
}

fn display_border(mut gizmos: Gizmos, transform_query: Query<&Transform, With<Border>>) {
    for transform in &transform_query {
        gizmos.ellipse_2d(
            transform.translation.xy(),
            0.,
            Vec2::new(CIRCLE_RADIUS + BORDER_OFFSET, CIRCLE_RADIUS + BORDER_OFFSET),
            Color::WHITE,
        );
    }
}

fn deselect(
    mut cmd: Commands,
    entities_query: Query<Entity, With<Selectable>>,
    mut deselect_event_reader: EventReader<DeselectEvent>,
) {
    if deselect_event_reader.read().next().is_none() {
        return;
    }

    for entity in &entities_query {
        cmd.entity(entity).remove::<(Selected, Border)>();
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldInspectorPlugin::new())
        .add_event::<FinishedSelectingEvent>()
        .add_event::<DeselectEvent>()
        .init_resource::<SelectionBox>()
        .add_systems(Startup, setup)
        .add_systems(Update, handle_mouse_input)
        .add_systems(Update, display_selection_box)
        .add_systems(Update, display_border)
        .add_systems(Update, deselect)
        .add_systems(Update, select_entities)
        .run();
}
