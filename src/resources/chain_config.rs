use screeps::ResourceType;

#[derive(Clone, Copy)]
pub struct FactoryChainConfig {
    pub limit: u32,
    pub chain: Chain,
    pub opt1: Option<Chain>,
    pub opt2: Option<Chain>,
}

#[derive(Clone, Copy)]
pub struct Chain {
    pub f_lvl: u8,
    pub resource: ResourceType,
    pub amount: u32
}

pub fn factory_chain_config(res: ResourceType) -> Option<FactoryChainConfig> {
    use ResourceType::*;

    Some(match res {
        // mechanical chain
        Metal => FactoryChainConfig {
            limit: 10_000,
            chain: Chain { f_lvl: 0, resource: Alloy, amount: 2_000 },
            opt1: None,
            opt2: None
        },
        Alloy => FactoryChainConfig {
            limit: 3_000,
            chain: Chain { f_lvl: 1, resource: Tube, amount: 150 },
            opt1: Some(Chain { f_lvl: 2, resource: Fixtures, amount: 70 }),
            opt2: None
        },
        Tube => FactoryChainConfig {
            limit: 40,
            chain: Chain { f_lvl: 3, resource: Frame, amount: 10 },
            opt1: None,
            opt2: None
        },
        Fixtures => FactoryChainConfig {
            limit: 20,
            chain: Chain { f_lvl: 3, resource: Frame, amount: 10 },
            opt1: None,
            opt2: None
        },
        // electroinal chain
        Silicon => FactoryChainConfig {
            limit: 10_000,
            chain: Chain { f_lvl: 0, resource: Wire, amount: 2_000 },
            opt1: None,
            opt2: None
        },
        Wire => FactoryChainConfig {
            limit: 3_000,
            chain: Chain { f_lvl: 1, resource: Switch, amount: 375 },
            opt1: Some(Chain { f_lvl: 2, resource: Transistor, amount: 20 }),
            opt2: Some(Chain { f_lvl: 3, resource: Microchip, amount: 10 }),
        },
        Switch => FactoryChainConfig {
            limit: 400,
            chain: Chain { f_lvl: 2, resource: Transistor, amount: 100 },
            opt1: None,
            opt2: None
        },
        Transistor => FactoryChainConfig {
            limit: 50,
            chain: Chain { f_lvl: 3, resource: Microchip, amount: 25 },
            opt1: None,
            opt2: None
        },
        // biological chain
        Biomass => FactoryChainConfig {
            limit: 10_000,
            chain: Chain { f_lvl: 0, resource: Cell, amount: 2_000 },
            opt1: None,
            opt2: None
        },
        Cell => FactoryChainConfig {
            limit: 3_000,
            chain: Chain { f_lvl: 1, resource: Phlegm, amount: 300 },
            opt1: None, //todo cell for tissue
            opt2: None
        },
        Phlegm => FactoryChainConfig {
            limit: 400,
            chain: Chain { f_lvl: 2, resource: Tissue, amount: 80 },
            opt1: None, //todo phlegm for muscle
            opt2: None
        },
        Tissue => FactoryChainConfig {
            limit: 99,
            chain: Chain { f_lvl: 3, resource: Muscle, amount: 33 },
            opt1: None, //todo tissue for organoid
            opt2: None
        },
        Muscle => FactoryChainConfig {
            limit: 10,
            chain: Chain { f_lvl: 4, resource: Organoid, amount: 10 },
            opt1: None,
            opt2: None
        },
        // mystical chain
        Mist => FactoryChainConfig {
            limit: 10_000,
            chain: Chain { f_lvl: 0, resource: Condensate, amount: 2_000 },
            opt1: None,
            opt2: None
        },
        Condensate => FactoryChainConfig {
            limit: 2_000,
            chain: Chain { f_lvl: 1, resource: Concentrate, amount: 300 },
            opt1: Some(Chain { f_lvl: 2, resource: Extract, amount: 200 }),
            opt2: None
        },
        Concentrate => FactoryChainConfig {
            limit: 300,
            chain: Chain { f_lvl: 2, resource: Extract, amount: 30 },
            opt1: Some(Chain { f_lvl: 3, resource: Spirit, amount: 10 }),
            opt2: None
        },
        Extract => FactoryChainConfig {
            limit: 40,
            chain: Chain { f_lvl: 3, resource: Spirit, amount: 20 },
            opt1: Some(Chain { f_lvl: 4, resource: Emanation, amount: 20 }),
            opt2: None
        },
        Spirit => FactoryChainConfig {
            limit: 16,
            chain: Chain { f_lvl: 4, resource: Emanation, amount: 8 },
            opt1: None,
            opt2: None
        },
        _ => return None,
    })
}