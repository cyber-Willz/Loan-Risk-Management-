use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Borrowers::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Borrowers::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Borrowers::Name).string().not_null())
                    .col(ColumnDef::new(Borrowers::BorrowerType).string_len(32).not_null())
                    .col(ColumnDef::new(Borrowers::NationalId).string().null())
                    .col(ColumnDef::new(Borrowers::Employer).string().null())
                    .col(ColumnDef::new(Borrowers::Address).string().null())
                    .col(ColumnDef::new(Borrowers::Email).string().null())
                    .col(ColumnDef::new(Borrowers::Phone).string().null())
                    .col(
                        ColumnDef::new(Borrowers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Borrowers::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_borrowers_national_id")
                    .table(Borrowers::Table)
                    .col(Borrowers::NationalId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Loans::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Loans::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Loans::BorrowerId).uuid().not_null())
                    .col(ColumnDef::new(Loans::PrincipalAmount).decimal_len(18, 2).not_null())
                    .col(ColumnDef::new(Loans::InterestRateBps).integer().not_null())
                    .col(ColumnDef::new(Loans::TermMonths).integer().not_null())
                    .col(ColumnDef::new(Loans::Status).string_len(32).not_null())
                    .col(ColumnDef::new(Loans::OriginationDate).date().not_null())
                    .col(ColumnDef::new(Loans::MaturityDate).date().not_null())
                    .col(ColumnDef::new(Loans::CollateralValue).decimal_len(18, 2).null())
                    .col(ColumnDef::new(Loans::Purpose).string().null())
                    .col(
                        ColumnDef::new(Loans::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Loans::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_loans_borrower")
                            .from(Loans::Table, Loans::BorrowerId)
                            .to(Borrowers::Table, Borrowers::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_loans_borrower_id")
                    .table(Loans::Table)
                    .col(Loans::BorrowerId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_loans_status")
                    .table(Loans::Table)
                    .col(Loans::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Payments::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Payments::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Payments::LoanId).uuid().not_null())
                    .col(ColumnDef::new(Payments::DueDate).date().not_null())
                    .col(ColumnDef::new(Payments::PaidDate).date().null())
                    .col(ColumnDef::new(Payments::AmountDue).decimal_len(18, 2).not_null())
                    .col(
                        ColumnDef::new(Payments::AmountPaid)
                            .decimal_len(18, 2)
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Payments::Status).string_len(32).not_null())
                    .col(ColumnDef::new(Payments::DaysLate).integer().null())
                    .col(
                        ColumnDef::new(Payments::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_payments_loan")
                            .from(Payments::Table, Payments::LoanId)
                            .to(Loans::Table, Loans::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_payments_loan_id")
                    .table(Payments::Table)
                    .col(Payments::LoanId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RelationshipLinks::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(RelationshipLinks::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(RelationshipLinks::SourceBorrowerId).uuid().not_null())
                    .col(ColumnDef::new(RelationshipLinks::TargetBorrowerId).uuid().not_null())
                    .col(ColumnDef::new(RelationshipLinks::RelationType).string_len(32).not_null())
                    .col(ColumnDef::new(RelationshipLinks::LoanId).uuid().null())
                    .col(
                        ColumnDef::new(RelationshipLinks::Weight)
                            .double()
                            .not_null()
                            .default(1.0),
                    )
                    .col(
                        ColumnDef::new(RelationshipLinks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_link_source")
                            .from(RelationshipLinks::Table, RelationshipLinks::SourceBorrowerId)
                            .to(Borrowers::Table, Borrowers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_link_target")
                            .from(RelationshipLinks::Table, RelationshipLinks::TargetBorrowerId)
                            .to(Borrowers::Table, Borrowers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_link_loan")
                            .from(RelationshipLinks::Table, RelationshipLinks::LoanId)
                            .to(Loans::Table, Loans::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_links_source")
                    .table(RelationshipLinks::Table)
                    .col(RelationshipLinks::SourceBorrowerId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_links_target")
                    .table(RelationshipLinks::Table)
                    .col(RelationshipLinks::TargetBorrowerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RiskAssessments::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(RiskAssessments::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(RiskAssessments::LoanId).uuid().not_null())
                    .col(
                        ColumnDef::new(RiskAssessments::AssessedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(RiskAssessments::State).string_len(32).not_null())
                    .col(ColumnDef::new(RiskAssessments::StateProbability).double().not_null())
                    .col(ColumnDef::new(RiskAssessments::Belief).json().not_null())
                    .col(
                        ColumnDef::new(RiskAssessments::NetworkContagionScore)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_risk_loan")
                            .from(RiskAssessments::Table, RiskAssessments::LoanId)
                            .to(Loans::Table, Loans::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_risk_loan_id")
                    .table(RiskAssessments::Table)
                    .col(RiskAssessments::LoanId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(NetworkSnapshots::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(NetworkSnapshots::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(NetworkSnapshots::SnapshotId).uuid().not_null())
                    .col(ColumnDef::new(NetworkSnapshots::BorrowerId).uuid().not_null())
                    .col(
                        ColumnDef::new(NetworkSnapshots::ComputedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(NetworkSnapshots::ClusterId).integer().not_null())
                    .col(ColumnDef::new(NetworkSnapshots::FiedlerComponent).double().not_null())
                    .col(ColumnDef::new(NetworkSnapshots::ContagionScore).double().not_null())
                    .col(ColumnDef::new(NetworkSnapshots::Degree).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_snapshot_borrower")
                            .from(NetworkSnapshots::Table, NetworkSnapshots::BorrowerId)
                            .to(Borrowers::Table, Borrowers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_snapshots_snapshot_id")
                    .table(NetworkSnapshots::Table)
                    .col(NetworkSnapshots::SnapshotId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(NetworkSnapshots::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(RiskAssessments::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(RelationshipLinks::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Payments::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Loans::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Borrowers::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Borrowers {
    Table,
    Id,
    Name,
    BorrowerType,
    NationalId,
    Employer,
    Address,
    Email,
    Phone,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Loans {
    Table,
    Id,
    BorrowerId,
    PrincipalAmount,
    InterestRateBps,
    TermMonths,
    Status,
    OriginationDate,
    MaturityDate,
    CollateralValue,
    Purpose,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Payments {
    Table,
    Id,
    LoanId,
    DueDate,
    PaidDate,
    AmountDue,
    AmountPaid,
    Status,
    DaysLate,
    CreatedAt,
}

#[derive(DeriveIden)]
enum RelationshipLinks {
    Table,
    Id,
    SourceBorrowerId,
    TargetBorrowerId,
    RelationType,
    LoanId,
    Weight,
    CreatedAt,
}

#[derive(DeriveIden)]
enum RiskAssessments {
    Table,
    Id,
    LoanId,
    AssessedAt,
    State,
    StateProbability,
    Belief,
    NetworkContagionScore,
}

#[derive(DeriveIden)]
enum NetworkSnapshots {
    Table,
    Id,
    SnapshotId,
    BorrowerId,
    ComputedAt,
    ClusterId,
    FiedlerComponent,
    ContagionScore,
    Degree,
}
