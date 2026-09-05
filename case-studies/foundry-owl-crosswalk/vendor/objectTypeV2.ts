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

import type { ObjectTypeV2 } from "@osdk/foundry.ontologies";

export const employeeObjectType: ObjectTypeV2 = {
  apiName: "Employee",
  displayName: "Employee",
  pluralDisplayName: "Employees",
  aliases: [],
  datasources: [],
  icon: { type: "blueprint", color: "blue", name: "person" },
  description: "A full-time or part-time \n\n employee of our firm",
  primaryKey: "employeeId",
  titleProperty: "fullName",
  visibility: "NORMAL",
  properties: {
    employeeId: {
      dataType: {
        type: "integer",
      },
      rid: "rid",
      typeClasses: [],
    },
    fullName: {
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
    office: {
      description:
        'The unique "ID" of the employee\'s \\"primary\\" assigned office.\n This is some more text.',
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
    class: {
      description: "",
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
    startDate: {
      description:
        "The date the employee was hired (most recently, if they were re-hired)",
      dataType: {
        type: "date",
      },
      rid: "rid",
      typeClasses: [],
    },
    employeeStatus: {
      description: "TimeSeries of the status of the employee",
      dataType: {
        type: "timeseries",
        itemType: {
          type: "string",
        },
      },
      rid: "rid",
      typeClasses: [],
    },
    employeeSensor: {
      description: "TimeSeries sensor of the status of the employee",
      dataType: {
        type: "timeseries",
        itemType: { type: "string" },
      },
      rid: "rid",
      typeClasses: [],
    },
    employeeLocation: {
      description: "Geotime series reference of the location of the employee",
      dataType: {
        type: "geotimeSeriesReference",
      },
      rid: "rid",
      typeClasses: [],
    },
    skillSet: {
      description: "The skills of the employee",
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
    skillSetEmbedding: {
      description: "Vectorized skill set",
      dataType: {
        type: "vector",
        dimension: 1536,
        supportsSearchWith: [{ value: "COSINE_SIMILARITY" }],
        embeddingModel: {
          type: "lms",
          value: "OPENAI_TEXT_EMBEDDING_ADA_002",
        },
      },
      rid: "rid",
      typeClasses: [],
    },
    favoriteRestaurants: {
      dataType: {
        type: "array",
        subType: {
          type: "string",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    employeeProfile: {
      description: "Employee profile with main value being the bio",
      dataType: {
        type: "struct",
        structFieldTypes: [
          {
            apiName: "bio",
            dataType: { type: "string" },
            rid: "ri.struct.bio",
            typeClasses: [],
          },
          {
            apiName: "yearsExperience",
            dataType: { type: "integer" },
            rid: "ri.struct.yearsExperience",
            typeClasses: [],
          },
        ],
        mainValue: {
          mainValueType: { type: "string" },
          fields: ["bio"],
        },
      },
      rid: "rid",
      typeClasses: [],
    },
    performanceScores: {
      description: "Array of performance scores with reducers",
      dataType: {
        type: "array",
        subType: { type: "double" },
        reducers: [{ direction: "DESCENDING_NULLS_LAST" }],
      },
      rid: "rid",
      typeClasses: [],
    },
  },
  rid: "ri.ontology.main.object-type.401ac022-89eb-4591-8b7e-0a912b9efb44",
  status: "ACTIVE",
} as const satisfies ObjectTypeV2;

export const officeObjectType: ObjectTypeV2 = {
  apiName: "Office",
  displayName: "Office",
  pluralDisplayName: "Office",
  aliases: [],
  datasources: [],
  icon: { type: "blueprint", color: "blue", name: "office" },
  description: "A office in our Company",
  primaryKey: "officeId",
  titleProperty: "officeId",
  properties: {
    officeId: {
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
    entrance: {
      dataType: {
        type: "geopoint",
      },
      rid: "rid",
      typeClasses: [],
    },
    occupiedArea: {
      description: "The occupied area of the Office",
      dataType: {
        type: "geoshape",
      },
      rid: "rid",
      typeClasses: [],
    },
    name: {
      description: "The Name of the Office",
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
    capacity: {
      description: "The capacity of the Office",
      dataType: {
        type: "integer",
      },
      rid: "rid",
      typeClasses: [],
    },
  },
  rid: "ri.ontology.main.object-type.404ac022-89eb-4591-8b7e-1a912b9efb45",
  status: "ACTIVE",
} as const satisfies ObjectTypeV2;

export const equipmentObjectType: ObjectTypeV2 = {
  apiName: "equipment",
  displayName: "Equipment",
  pluralDisplayName: "Equipment",
  aliases: [],
  datasources: [],
  icon: { type: "blueprint", color: "blue", name: "gear" },
  primaryKey: "equipmentId",
  titleProperty: "type",
  properties: {
    equipmentId: {
      description: "The id of an equipment",
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
    type: {
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
  },
  rid: "ri.ontology.main.object-type.808ac022-89eb-4591-8b7e-1a912b9efb45",
  status: "ACTIVE",
};

export const objectTypeWithTimestampPrimaryKey: ObjectTypeV2 = {
  apiName: "ObjectWithTimestampPrimaryKey",
  displayName: "ObjectWithTimestampPrimaryKey",
  pluralDisplayName: "ObjectWithTimestampPrimaryKeys",
  aliases: [],
  datasources: [],
  icon: { type: "blueprint", color: "blue", name: "person" },
  description: "Object Type With Timestamp Primary Key",
  primaryKey: "timestamp",
  titleProperty: "value",
  properties: {
    timestamp: {
      dataType: {
        type: "timestamp",
      },
      rid: "rid",
      typeClasses: [],
    },
    value: {
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
  },
  rid: "ri.ontology.main.object-type.32bb9402-9cab-4705-81bb-edb65a4a6806",
  status: "ACTIVE",
};

export const objectTypeWithAllPropertyTypes: ObjectTypeV2 = {
  apiName: "objectTypeWithAllPropertyTypes",
  displayName: "objectTypeWithAllPropertyType",
  pluralDisplayName: "objectTypeWithAllPropertyTypes",
  aliases: [],
  datasources: [],
  icon: { type: "blueprint", color: "blue", name: "person" },
  description: "An object type with all property types",
  primaryKey: "id",
  titleProperty: "string",
  properties: {
    id: {
      dataType: {
        type: "integer",
      },
      rid: "rid",
      typeClasses: [],
    },
    string: {
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
    boolean: {
      dataType: {
        type: "boolean",
      },
      rid: "rid",
      typeClasses: [],
    },
    date: {
      dataType: {
        type: "date",
      },
      rid: "rid",
      typeClasses: [],
    },
    dateTime: {
      dataType: {
        type: "timestamp",
      },
      rid: "rid",
      typeClasses: [],
    },
    decimal: {
      dataType: {
        type: "decimal",
      },
      rid: "rid",
      typeClasses: [],
    },
    integer: {
      dataType: {
        type: "integer",
      },
      rid: "rid",
      typeClasses: [],
    },
    long: {
      dataType: {
        type: "long",
      },
      rid: "rid",
      typeClasses: [],
    },
    short: {
      dataType: {
        type: "short",
      },
      rid: "rid",
      typeClasses: [],
    },
    float: {
      dataType: {
        type: "float",
      },
      rid: "rid",
      typeClasses: [],
    },
    double: {
      dataType: {
        type: "double",
      },
      rid: "rid",
      typeClasses: [],
    },
    byte: {
      dataType: {
        type: "byte",
      },
      rid: "rid",
      typeClasses: [],
    },
    attachment: {
      dataType: {
        type: "attachment",
      },
      rid: "rid",
      typeClasses: [],
    },
    attachment2: {
      dataType: {
        type: "attachment",
      },
      rid: "rid",
      typeClasses: [],
    },
    geoPoint: {
      dataType: {
        type: "geopoint",
      },
      rid: "rid",
      typeClasses: [],
    },
    geoShape: {
      dataType: {
        type: "geoshape",
      },
      rid: "rid",
      typeClasses: [],
    },
    stringArray: {
      dataType: {
        type: "array",
        subType: {
          type: "string",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    booleanArray: {
      dataType: {
        type: "array",
        subType: {
          type: "boolean",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    dateArray: {
      dataType: {
        type: "array",
        subType: {
          type: "date",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    dateTimeArray: {
      dataType: {
        type: "array",
        subType: {
          type: "timestamp",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    decimalArray: {
      dataType: {
        type: "array",
        subType: {
          type: "decimal",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    integerArray: {
      dataType: {
        type: "array",
        subType: {
          type: "integer",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    longArray: {
      dataType: {
        type: "array",
        subType: {
          type: "long",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    shortArray: {
      dataType: {
        type: "array",
        subType: {
          type: "short",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    floatArray: {
      dataType: {
        type: "array",
        subType: {
          type: "float",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    doubleArray: {
      dataType: {
        type: "array",
        subType: {
          type: "double",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    byteArray: {
      dataType: {
        type: "array",
        subType: {
          type: "byte",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    attachmentArray: {
      dataType: {
        type: "array",
        subType: {
          type: "attachment",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    geoPointArray: {
      dataType: {
        type: "array",
        subType: {
          type: "geopoint",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    geoShapeArray: {
      dataType: {
        type: "array",
        subType: {
          type: "geoshape",
        },
        reducers: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    mediaReference: {
      dataType: {
        type: "mediaReference",
      },
      rid: "rid",
      typeClasses: [],
    },
    vector: {
      dataType: {
        type: "vector",
        dimension: 100,
        supportsSearchWith: [],
      },
      rid: "rid",
      typeClasses: [],
    },
    cipherText: {
      dataType: {
        type: "cipherText",
      },
      rid: "rid",
      typeClasses: [],
    },
  },
  rid: "ri.ontology.main.object-type.401ac022-89eb-4591-8b7e-0a912b9efb44",
  status: "ACTIVE",
};

export const ObjectTypesV2: ObjectTypeV2[] = [
  employeeObjectType,
  objectTypeWithAllPropertyTypes,
  objectTypeWithTimestampPrimaryKey,
  officeObjectType,
  equipmentObjectType,
];

export const taskObjectType: ObjectTypeV2 = {
  apiName: "Task",
  displayName: "Task",
  pluralDisplayName: "Tasks",
  aliases: [],
  datasources: [],
  icon: { type: "blueprint", color: "blue", name: "box" },
  primaryKey: "id",
  properties: {
    id: {
      dataType: {
        type: "integer",
      },
      rid: "rid",
      typeClasses: [],
    },

    name: {
      dataType: { type: "string" },
      rid: "rid",
      typeClasses: [],
    },
  },
  status: "ACTIVE",
  rid: "ri.task",
  titleProperty: "name",
};

export const todoObjectType: ObjectTypeV2 = {
  apiName: "Todo",
  displayName: "Todo",
  pluralDisplayName: "ManyTodo",
  aliases: [],
  datasources: [],
  icon: { type: "blueprint", color: "blue", name: "box" },
  primaryKey: "id",
  properties: {
    id: {
      dataType: {
        type: "integer",
      },
      rid: "rid",
      typeClasses: [],
    },
    text: {
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
  },
  rid: "ri.todo",
  status: "ACTIVE",
  titleProperty: "text",
};

export const personObjectType: ObjectTypeV2 = {
  apiName: "Person",
  displayName: "Person",
  pluralDisplayName: "Persons",
  aliases: [],
  datasources: [],
  icon: { type: "blueprint", color: "blue", name: "person" },
  primaryKey: "id",
  properties: {
    id: {
      dataType: {
        type: "integer",
      },
      rid: "rid",
      typeClasses: [],
    },
    name: {
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
  },
  rid: "ri.person",
  status: "ACTIVE",
  titleProperty: "name",
};

export const BGaoNflPlayerObjectType: ObjectTypeV2 = {
  apiName: "BgaoNflPlayer",
  displayName: "id",
  pluralDisplayName: "BgaoNflPlayers",
  aliases: [],
  datasources: [],
  icon: { type: "blueprint", color: "blue", name: "box" },
  primaryKey: "id",
  properties: {
    id: {
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
    gamesPlayed: {
      dataType: {
        type: "integer",
      },
      rid: "rid",
      typeClasses: [],
    },
    name: {
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
    number: {
      dataType: {
        type: "integer",
      },
      rid: "rid",
      typeClasses: [],
    },
    wikiUrl: {
      dataType: {
        type: "string",
      },
      rid: "rid",
      typeClasses: [],
    },
    address: {
      dataType: {
        type: "struct",
        structFieldTypes: [
          {
            apiName: "addressLine1",
            dataType: {
              type: "string",
            },
            rid: "ri.a.b.c.d",
            typeClasses: [],
          },
          {
            apiName: "addressLine2",
            dataType: {
              type: "string",
            },
            rid: "ri.a.b.c.d",
            typeClasses: [],
          },
          {
            apiName: "city",
            dataType: {
              type: "string",
            },
            rid: "ri.a.b.c.d",
            typeClasses: [],
          },
          {
            apiName: "state",
            dataType: {
              type: "string",
            },
            rid: "ri.a.b.c.d",
            typeClasses: [],
          },
          {
            apiName: "zipCode",
            dataType: {
              type: "integer",
            },
            rid: "ri.a.b.c.d",
            typeClasses: [],
          },
        ],
      },
      rid: "rid",
      typeClasses: [],
    },
    addressArray: {
      dataType: {
        type: "array",
        subType: {
          type: "struct",
          structFieldTypes: [
            {
              apiName: "addressLine1",
              dataType: {
                type: "string",
              },
              rid: "ri.a.b.c.d",
              typeClasses: [],
            },
            {
              apiName: "addressLine2",
              dataType: {
                type: "string",
              },
              rid: "ri.a.b.c.d",
              typeClasses: [],
            },
            {
              apiName: "city",
              dataType: {
                type: "geoshape",
              },
              rid: "ri.a.b.c.d",
              typeClasses: [],
            },
            {
              apiName: "location",
              dataType: {
                type: "geopoint",
              },
              rid: "ri.a.b.c.d",
              typeClasses: [],
            },
            {
              apiName: "zipCode",
              dataType: {
                type: "integer",
              },
              rid: "ri.a.b.c.d",
              typeClasses: [],
            },
          ],
        },
        reducers: [],
      },
      rid: "ri.a.b.c.structArray",
      typeClasses: [],
    },
  },
  rid: "ri.nflplayer",
  status: "ACTIVE",
  titleProperty: "name",
};
