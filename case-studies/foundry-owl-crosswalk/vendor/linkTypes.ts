/*
 * Copyright 2023 Palantir Technologies, Inc. All rights reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

import type { LinkTypeSideV2 } from "@osdk/foundry.ontologies";

import {
  employeeObjectType,
  objectTypeWithAllPropertyTypes,
  officeObjectType,
} from "./objectTypes.js";

export const peepsLinkType: LinkTypeSideV2 = {
  apiName: "peeps",
  status: "EXPERIMENTAL",
  objectTypeApiName: employeeObjectType.apiName,
  cardinality: "MANY",
  displayName: "Peeps",
  linkTypeRid: "rid.link-type.327",
};

export const leadLinkType: LinkTypeSideV2 = {
  apiName: "lead",
  status: "EXPERIMENTAL",
  objectTypeApiName: employeeObjectType.apiName,
  cardinality: "ONE",
  displayName: "Lead",
  linkTypeRid: peepsLinkType.linkTypeRid,
  foreignKeyPropertyApiName: "leadId",
};

export const visitedOffices: LinkTypeSideV2 = {
  apiName: "visitedOffices",
  status: "EXPERIMENTAL",
  objectTypeApiName: officeObjectType.apiName,
  cardinality: "MANY",
  displayName: "Visited Offices",
  linkTypeRid: "ri.a.b.c.visitedOffices",
};

export const officeLinkType: LinkTypeSideV2 = {
  apiName: "officeLink",
  status: "EXPERIMENTAL",
  objectTypeApiName: officeObjectType.apiName,
  cardinality: "ONE",
  displayName: "Office",
  linkTypeRid: "rid.link-type.324",
  foreignKeyPropertyApiName: "office",
};

export const occupants: LinkTypeSideV2 = {
  apiName: "occupants",
  status: "EXPERIMENTAL",
  objectTypeApiName: employeeObjectType.apiName,
  cardinality: "MANY",
  displayName: "Office Occupants",
  linkTypeRid: officeLinkType.linkTypeRid,
};

export const allPropertyTypesSelfLink: LinkTypeSideV2 = {
  apiName: "linkedObjectType",
  status: "EXPERIMENTAL",
  objectTypeApiName: objectTypeWithAllPropertyTypes.apiName,
  cardinality: "ONE",
  displayName: "Linked Object Type",
  linkTypeRid: "rid.link-type.832",
};
