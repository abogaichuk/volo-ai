use screeps::{
    Position, RawObjectId, Transferable, StructureExtension, StructureTower,
    StructureSpawn, ResourceType, HasPosition, HasId
};

pub mod claimed;
pub mod farm;
pub mod neutral;

pub trait Fillable {
    fn position(&self) -> Position;
    fn id(&self) -> RawObjectId;
    fn free_capacity(&self) -> i32;
    fn as_transferable(&self) -> &dyn Transferable;
}

impl Fillable for StructureExtension {
    fn position(&self) -> Position {
        self.pos()
    }

    fn id(&self) -> RawObjectId {
        self.raw_id()
    }

    fn as_transferable(&self) -> &dyn Transferable {
        self
    }
    
    fn free_capacity(&self) -> i32 {
        self.store().get_free_capacity(Some(ResourceType::Energy))
    }
}

impl Fillable for StructureTower {
    fn position(&self) -> Position {
        self.pos()
    }

    fn id(&self) -> RawObjectId {
        self.raw_id()
    }

    fn as_transferable(&self) -> &dyn Transferable {
        self
    }

    fn free_capacity(&self) -> i32 {
        self.store().get_free_capacity(Some(ResourceType::Energy))
    }
}

impl Fillable for StructureSpawn {
    fn position(&self) -> Position {
        self.pos()
    }

    fn id(&self) -> RawObjectId {
        self.raw_id()
    }

    fn as_transferable(&self) -> &dyn Transferable {
        self
    }

    fn free_capacity(&self) -> i32 {
        self.store().get_free_capacity(Some(ResourceType::Energy))
    }
}