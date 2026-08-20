pub mod borrower;
pub mod loan;
pub mod network_snapshot;
pub mod payment;
pub mod relationship_link;
pub mod risk_assessment;

pub mod prelude {
    pub use super::borrower::{
        BorrowerType, Entity as Borrower, Model as BorrowerModel,
    };
    pub use super::loan::{Entity as Loan, LoanStatus, Model as LoanModel};
    pub use super::network_snapshot::{Entity as NetworkSnapshot, Model as NetworkSnapshotModel};
    pub use super::payment::{Entity as Payment, Model as PaymentModel, PaymentStatus};
    pub use super::relationship_link::{
        Entity as RelationshipLink, Model as RelationshipLinkModel, RelationType,
    };
    pub use super::risk_assessment::{
        Entity as RiskAssessment, Model as RiskAssessmentModel, RiskState,
    };
}
