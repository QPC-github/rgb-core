// RGB Core Library: consensus layer for RGB smart contracts.
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2019-2023 by
//     Dr Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2019-2023 LNP/BP Standards Association. All rights reserved.
// Copyright (C) 2019-2023 Dr Maxim Orlovsky. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use strict_types::TypeSystem;

use crate::schema::AssignmentType;
use crate::{
    validation, Assign, ConfidentialState, ExposedSeal, ExposedState, OpId, StateCommitment,
    StateData, StateSchema,
};

impl StateSchema {
    pub fn validate<State: ExposedState, Seal: ExposedSeal>(
        &self,
        type_system: &TypeSystem,
        opid: &OpId,
        state_type: AssignmentType,
        data: &Assign<State, Seal>,
    ) -> validation::Status {
        let mut status = validation::Status::new();
        match data {
            Assign::Confidential { state, .. } | Assign::ConfidentialState { state, .. } => {
                match (self, state.state_commitment()) {
                    (StateSchema::Declarative, StateCommitment::Void) => {}
                    (StateSchema::Fungible(_), StateCommitment::Fungible(value)) => {
                        // [SECURITY-CRITICAL]: Bulletproofs validation
                        if let Err(err) = value.verify_range_proof() {
                            status.add_failure(validation::Failure::BulletproofsInvalid(
                                *opid,
                                state_type,
                                err.to_string(),
                            ));
                        }
                    }
                    (StateSchema::Structured(_), StateCommitment::Structured(_)) => {
                        status.add_info(validation::Info::UncheckableConfidentialState(
                            *opid, state_type,
                        ));
                    }
                    (StateSchema::Attachment(_), StateCommitment::Attachment(_)) => {
                        status.add_info(validation::Info::UncheckableConfidentialState(
                            *opid, state_type,
                        ));
                    }
                    // all other options are mismatches
                    (state_schema, found) => {
                        status.add_failure(validation::Failure::StateTypeMismatch {
                            opid: *opid,
                            state_type,
                            expected: state_schema.state_type(),
                            found: found.state_type(),
                        });
                    }
                }
            }
            Assign::Revealed { state, .. } | Assign::ConfidentialSeal { state, .. } => {
                match (self, state.state_data()) {
                    (StateSchema::Declarative, StateData::Void) => {}
                    (StateSchema::Attachment(media_type), StateData::Attachment(attach))
                        if !attach.media_type.conforms(media_type) =>
                    {
                        status.add_failure(validation::Failure::MediaTypeMismatch {
                            opid: *opid,
                            state_type,
                            expected: media_type.clone(),
                            found: attach.media_type,
                        });
                    }
                    (StateSchema::Fungible(schema), StateData::Fungible(v))
                        if v.value.fungible_type() != *schema =>
                    {
                        status.add_failure(validation::Failure::FungibleTypeMismatch {
                            opid: *opid,
                            state_type,
                            expected: *schema,
                            found: v.value.fungible_type(),
                        });
                    }
                    (StateSchema::Fungible(_), StateData::Fungible(_)) => {}
                    (StateSchema::Structured(sem_id), StateData::Structured(data)) => {
                        if type_system
                            .strict_deserialize_type(*sem_id, data.as_ref())
                            .is_err()
                        {
                            status.add_failure(validation::Failure::SchemaInvalidOwnedValue(
                                *opid, state_type, *sem_id,
                            ));
                        };
                    }
                    // all other options are mismatches
                    (state_schema, found) => {
                        status.add_failure(validation::Failure::StateTypeMismatch {
                            opid: *opid,
                            state_type,
                            expected: state_schema.state_type(),
                            found: found.state_type(),
                        });
                    }
                }
            }
        }
        status
    }
}
