pub enum Weapon {
    Missile,
    Ray,
}
impl Weapon {
    pub fn name(&self) -> String {
        match self {
            Weapon::Missile => "Missile".to_string(),
            Weapon::Ray => "Ray".to_string(),
        }
    }
    pub fn next(&mut self) {
        *self = match self {
            Weapon::Missile => Weapon::Ray,
            Weapon::Ray => Weapon::Missile,
        }
    }
}
