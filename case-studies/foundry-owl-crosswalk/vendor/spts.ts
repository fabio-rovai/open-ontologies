/*
 * Copyright 2024 Palantir Technologies, Inc. All rights reserved.
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

import type { SharedPropertyType } from "@osdk/foundry.ontologies";

export const fooSpt: SharedPropertyType = {
  rid: "ri.sharedPropertyType.main.sharedPropertyType.1",
  apiName: "fooSpt",
  dataType: {
    type: "string",
  },
  displayName: "Foo",
  description: "A foo",
  typeClasses: [],
} satisfies SharedPropertyType;
