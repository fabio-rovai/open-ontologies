#  Copyright 2024 Palantir Technologies, Inc.
#
#  Licensed under the Apache License, Version 2.0 (the "License");
#  you may not use this file except in compliance with the License.
#  You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.


from __future__ import annotations

import typing

import pydantic
import typing_extensions

from foundry_sdk import _core as core


class AnyType(core.ModelBase):
    """AnyType"""

    type: typing.Literal["any"] = "any"


class ArrayFieldType(core.ModelBase):
    """ArrayFieldType"""

    items_schema: FieldSchema = pydantic.Field(alias=str("itemsSchema"))  # type: ignore[literal-required]
    type: typing.Literal["array"] = "array"


class AttachmentType(core.ModelBase):
    """AttachmentType"""

    type: typing.Literal["attachment"] = "attachment"


Attribution: typing_extensions.TypeAlias = str
"""Attribution for a request"""


class BinaryType(core.ModelBase):
    """BinaryType"""

    type: typing.Literal["binary"] = "binary"


class BooleanType(core.ModelBase):
    """BooleanType"""

    type: typing.Literal["boolean"] = "boolean"


class BranchMetadata(core.ModelBase):
    """Metadata about a Foundry branch."""

    rid: FoundryBranch


BranchName: typing_extensions.TypeAlias = str
"""The name of a Branch."""


BuildRid: typing_extensions.TypeAlias = core.RID
"""The RID of a Build."""


class ByteType(core.ModelBase):
    """ByteType"""

    type: typing.Literal["byte"] = "byte"


CheckReportRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier (RID) of a Data Health Check Report."""


CheckRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier (RID) of a Data Health Check."""


class CipherTextType(core.ModelBase):
    """CipherTextType"""

    default_cipher_channel: typing.Optional[core.RID] = pydantic.Field(alias=str("defaultCipherChannel"), default=None)  # type: ignore[literal-required]
    """An optional Cipher Channel RID which can be used for encryption updates to empty values."""

    type: typing.Literal["cipherText"] = "cipherText"


Color: typing_extensions.TypeAlias = str
"""The hex value of a color."""


ColumnName: typing_extensions.TypeAlias = str
"""The name of a column in a dataset."""


ComputeSeconds: typing_extensions.TypeAlias = float
"""A measurement of compute usage expressed in [compute-seconds](https://palantir.com/docs/foundry/resource-management/usage-types#compute-second). For more information, please refer to the [Usage types](https://palantir.com/docs/foundry/resource-management/usage-types) documentation."""


ContentLength: typing_extensions.TypeAlias = core.Long
"""ContentLength"""


ContentType: typing_extensions.TypeAlias = str
"""ContentType"""


CreatedTime: typing_extensions.TypeAlias = core.AwareDatetime
"""The time at which the resource was created."""


CustomMetadata: typing_extensions.TypeAlias = typing.Dict[str, typing.Any]
"""CustomMetadata"""


class DatasetFieldSchema(core.ModelBase):
    """A field in a Foundry dataset."""

    type: SchemaFieldType
    name: typing.Optional[FieldName] = None
    """The name of a column. May be absent in nested schema objects."""

    nullable: bool
    """Indicates whether values of this field may be null."""

    user_defined_type_class: typing.Optional[str] = pydantic.Field(alias=str("userDefinedTypeClass"), default=None)  # type: ignore[literal-required]
    """Canonical classname of the user-defined type for this field. This should be a subclass of Spark's `UserDefinedType`."""

    custom_metadata: typing.Optional[CustomMetadata] = pydantic.Field(alias=str("customMetadata"), default=None)  # type: ignore[literal-required]
    """User-supplied custom metadata about the column, such as Foundry web archetypes, descriptions, etc."""

    array_subtype: typing.Optional[DatasetFieldSchema] = pydantic.Field(alias=str("arraySubtype"), default=None)  # type: ignore[literal-required]
    """Only used when field type is array."""

    precision: typing.Optional[int] = None
    """Only used when field type is decimal."""

    scale: typing.Optional[int] = None
    """Only used when field type is decimal."""

    map_key_type: typing.Optional[DatasetFieldSchema] = pydantic.Field(alias=str("mapKeyType"), default=None)  # type: ignore[literal-required]
    """Only used when field type is map."""

    map_value_type: typing.Optional[DatasetFieldSchema] = pydantic.Field(alias=str("mapValueType"), default=None)  # type: ignore[literal-required]
    """Only used when field type is map."""

    sub_schemas: typing.Optional[typing.List[DatasetFieldSchema]] = pydantic.Field(alias=str("subSchemas"), default=None)  # type: ignore[literal-required]
    """Only used when field type is struct."""


DatasetRid: typing_extensions.TypeAlias = core.RID
"""The Resource Identifier (RID) of a Dataset."""


class DatasetSchema(core.ModelBase):
    """The schema for a Foundry dataset. Files uploaded to this dataset must match this schema."""

    field_schema_list: typing.List[DatasetFieldSchema] = pydantic.Field(alias=str("fieldSchemaList"))  # type: ignore[literal-required]


class DateType(core.ModelBase):
    """DateType"""

    type: typing.Literal["date"] = "date"


class DecimalType(core.ModelBase):
    """DecimalType"""

    precision: typing.Optional[int] = None
    """The total number of digits of the Decimal type. The maximum value is 38."""

    scale: typing.Optional[int] = None
    """The number of digits to the right of the decimal point. The maximum value is 38."""

    type: typing.Literal["decimal"] = "decimal"


DisplayName: typing_extensions.TypeAlias = str
"""The display name of the entity."""


class Distance(core.ModelBase):
    """A measurement of distance."""

    value: float
    unit: DistanceUnit


DistanceUnit: typing_extensions.TypeAlias = typing.Literal[
    "MILLIMETERS",
    "CENTIMETERS",
    "METERS",
    "KILOMETERS",
    "INCHES",
    "FEET",
    "YARDS",
    "MILES",
    "NAUTICAL_MILES",
]
"""DistanceUnit"""


class DoubleType(core.ModelBase):
    """DoubleType"""

    type: typing.Literal["double"] = "double"


class Duration(core.ModelBase):
    """A measurement of duration."""

    value: int
    """The duration value."""

    unit: TimeUnit
    """The unit of duration."""


DurationSeconds: typing_extensions.TypeAlias = core.Long
"""A duration of time measured in seconds."""


EmbeddingModel: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["LmsEmbeddingModel", "FoundryLiveDeployment"], pydantic.Field(discriminator="type")
]
"""EmbeddingModel"""


EnrollmentRid: typing_extensions.TypeAlias = core.RID
"""EnrollmentRid"""


class Field(core.ModelBase):
    """
    A field in a Foundry schema. For more information on supported data types, see the
    [supported field types](https://palantir.com/docs/foundry/data-integration/datasets/#supported-field-types) user documentation.
    """

    name: FieldName
    schema_: FieldSchema = pydantic.Field(alias=str("schema"))  # type: ignore[literal-required]


FieldDataType: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "StructFieldType",
        "DateType",
        "StringType",
        "ByteType",
        "DoubleType",
        "IntegerType",
        "FloatType",
        "LongType",
        "BooleanType",
        "ArrayFieldType",
        "BinaryType",
        "ShortType",
        "DecimalType",
        "MapFieldType",
        "TimestampType",
    ],
    pydantic.Field(discriminator="type"),
]
"""FieldDataType"""


FieldName: typing_extensions.TypeAlias = str
"""FieldName"""


class FieldSchema(core.ModelBase):
    """The specification of the type of a Foundry schema field."""

    nullable: bool
    custom_metadata: typing.Optional[CustomMetadata] = pydantic.Field(alias=str("customMetadata"), default=None)  # type: ignore[literal-required]
    data_type: FieldDataType = pydantic.Field(alias=str("dataType"))  # type: ignore[literal-required]


FilePath: typing_extensions.TypeAlias = str
"""
The path to a File within Foundry. Paths are relative and must not start with a leading slash.
Examples: `my-file.txt`, `path/to/my-file.jpg`, `dataframe.snappy.parquet`.
"""


Filename: typing_extensions.TypeAlias = str
"""The name of a File within Foundry. Examples: `my-file.txt`, `my-file.jpg`, `dataframe.snappy.parquet`."""


class FilterBinaryType(core.ModelBase):
    """FilterBinaryType"""

    type: typing.Literal["binary"] = "binary"


class FilterBooleanType(core.ModelBase):
    """FilterBooleanType"""

    type: typing.Literal["boolean"] = "boolean"


class FilterDateTimeType(core.ModelBase):
    """FilterDateTimeType"""

    type: typing.Literal["dateTime"] = "dateTime"


class FilterDateType(core.ModelBase):
    """FilterDateType"""

    type: typing.Literal["date"] = "date"


class FilterDoubleType(core.ModelBase):
    """FilterDoubleType"""

    type: typing.Literal["double"] = "double"


class FilterEnumType(core.ModelBase):
    """FilterEnumType"""

    values: typing.List[str]
    """The values allowed by the enum type."""

    type: typing.Literal["enum"] = "enum"


class FilterFloatType(core.ModelBase):
    """FilterFloatType"""

    type: typing.Literal["float"] = "float"


class FilterIntegerType(core.ModelBase):
    """FilterIntegerType"""

    type: typing.Literal["integer"] = "integer"


class FilterLongType(core.ModelBase):
    """FilterLongType"""

    type: typing.Literal["long"] = "long"


class FilterRidType(core.ModelBase):
    """FilterRidType"""

    type: typing.Literal["rid"] = "rid"


class FilterStringType(core.ModelBase):
    """FilterStringType"""

    type: typing.Literal["string"] = "string"


FilterType: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "FilterDateTimeType",
        "FilterDateType",
        "FilterBooleanType",
        "FilterStringType",
        "FilterDoubleType",
        "FilterBinaryType",
        "FilterIntegerType",
        "FilterFloatType",
        "FilterRidType",
        "FilterUuidType",
        "FilterEnumType",
        "FilterLongType",
    ],
    pydantic.Field(discriminator="type"),
]
"""FilterType"""


class FilterUuidType(core.ModelBase):
    """FilterUuidType"""

    type: typing.Literal["uuid"] = "uuid"


class FloatType(core.ModelBase):
    """FloatType"""

    type: typing.Literal["float"] = "float"


FolderRid: typing_extensions.TypeAlias = core.RID
"""FolderRid"""


FoundryBranch: typing_extensions.TypeAlias = str
"""The Foundry branch identifier, specifically its rid. Different identifier types may be used in the future as values."""


class FoundryLiveDeployment(core.ModelBase):
    """FoundryLiveDeployment"""

    rid: typing.Optional[core.RID] = None
    """The live deployment identifier. This rid is of the format 'ri.foundry-ml-live.main.live-deployment.<uuid>'."""

    input_param_name: typing.Optional[str] = pydantic.Field(alias=str("inputParamName"), default=None)  # type: ignore[literal-required]
    """The name of the input parameter to the model which should contain the query string."""

    output_param_name: typing.Optional[str] = pydantic.Field(alias=str("outputParamName"), default=None)  # type: ignore[literal-required]
    """The name of the output parameter to the model which should contain the computed embedding."""

    type: typing.Literal["foundryLiveDeployment"] = "foundryLiveDeployment"


class FullRowChangeDataCaptureConfiguration(core.ModelBase):
    """
    Configuration for change data capture which resolves the latest state of the dataset based on new full rows
    being pushed to the stream. For example, if a value for a row is updated, it is only sufficient to publish
    the entire new state of that row to the stream.
    """

    deletion_field_name: FieldName = pydantic.Field(alias=str("deletionFieldName"))  # type: ignore[literal-required]
    """The name of a boolean field in the schema that indicates whether or not a row has been deleted."""

    ordering_field_name: FieldName = pydantic.Field(alias=str("orderingFieldName"))  # type: ignore[literal-required]
    """
    The name of an ordering field that determines the newest state for a row in the dataset. 

    The ordering field can only be of the following types:
    - Byte
    - Date
    - Decimal
    - Integer
    - Long
    - Short
    - String
    - Timestamp
    """

    type: typing.Literal["fullRow"] = "fullRow"


class GeoPointType(core.ModelBase):
    """GeoPointType"""

    type: typing.Literal["geopoint"] = "geopoint"


class GeoShapeType(core.ModelBase):
    """GeoShapeType"""

    type: typing.Literal["geoshape"] = "geoshape"


class GeohashType(core.ModelBase):
    """GeohashType"""

    type: typing.Literal["geohash"] = "geohash"


class GeotimeSeriesReferenceType(core.ModelBase):
    """GeotimeSeriesReferenceType"""

    type: typing.Literal["geotimeSeriesReference"] = "geotimeSeriesReference"


GroupId: typing_extensions.TypeAlias = core.UUID
"""A Foundry Group ID."""


GroupName: typing_extensions.TypeAlias = str
"""The display name of a multipass group."""


GroupRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier (RID) of a multipass group."""


IncludeComputeUsage: typing_extensions.TypeAlias = bool
"""
Indicates whether the response should include compute usage details for the request. This feature is currently
only available for OSDK applications.
Note: Enabling this flag may slow down query performance and is not recommended for use in production.
"""


class IntegerType(core.ModelBase):
    """IntegerType"""

    type: typing.Literal["integer"] = "integer"


JobRid: typing_extensions.TypeAlias = core.RID
"""The RID of a Job."""


class LmsEmbeddingModel(core.ModelBase):
    """A model provided by Language Model Service."""

    value: LmsEmbeddingModelValue
    type: typing.Literal["lms"] = "lms"


LmsEmbeddingModelValue: typing_extensions.TypeAlias = typing.Literal[
    "OPENAI_TEXT_EMBEDDING_ADA_002",
    "TEXT_EMBEDDING_3_LARGE",
    "TEXT_EMBEDDING_3_SMALL",
    "SNOWFLAKE_ARCTIC_EMBED_M",
    "INSTRUCTOR_LARGE",
    "BGE_BASE_EN_V1_5",
]
"""LmsEmbeddingModelValue"""


class LongType(core.ModelBase):
    """LongType"""

    type: typing.Literal["long"] = "long"


class MapFieldType(core.ModelBase):
    """MapFieldType"""

    key_schema: FieldSchema = pydantic.Field(alias=str("keySchema"))  # type: ignore[literal-required]
    value_schema: FieldSchema = pydantic.Field(alias=str("valueSchema"))  # type: ignore[literal-required]
    type: typing.Literal["map"] = "map"


MarkingId: typing_extensions.TypeAlias = str
"""The ID of a security marking."""


class MarkingType(core.ModelBase):
    """MarkingType"""

    marking_type: typing.Optional[MarkingTypeValue] = pydantic.Field(alias=str("markingType"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["marking"] = "marking"


MarkingTypeValue: typing_extensions.TypeAlias = typing.Literal["CBAC", "MANDATORY"]
"""
The kind of marking applied by a marking property type.
- `CBAC`: Classification-based access control markings.
- `MANDATORY`: Standard non-classification markings. Example - Organizations.
"""


MediaItemPath: typing_extensions.TypeAlias = str
"""
A user-specified identifier for a media item within a media set.
Paths must be less than 256 characters long.
If multiple items are written to the same media set at the same path, then when retrieving by path the media
item which was written last is returned.
"""


MediaItemReadToken: typing_extensions.TypeAlias = str
"""A token that grants access to read specific media items."""


MediaItemRid: typing_extensions.TypeAlias = core.RID
"""The Resource Identifier (RID) of an individual Media Item within a Media Set in Foundry."""


class MediaReference(core.ModelBase):
    """The representation of a media reference."""

    mime_type: MediaType = pydantic.Field(alias=str("mimeType"))  # type: ignore[literal-required]
    reference: Reference


class MediaReferenceType(core.ModelBase):
    """MediaReferenceType"""

    type: typing.Literal["mediaReference"] = "mediaReference"


MediaSetRid: typing_extensions.TypeAlias = core.RID
"""The Resource Identifier (RID) of a Media Set in Foundry."""


class MediaSetViewItem(core.ModelBase):
    """MediaSetViewItem"""

    media_set_rid: MediaSetRid = pydantic.Field(alias=str("mediaSetRid"))  # type: ignore[literal-required]
    media_set_view_rid: MediaSetViewRid = pydantic.Field(alias=str("mediaSetViewRid"))  # type: ignore[literal-required]
    media_item_rid: MediaItemRid = pydantic.Field(alias=str("mediaItemRid"))  # type: ignore[literal-required]
    token: typing.Optional[MediaItemReadToken] = None


class MediaSetViewItemWrapper(core.ModelBase):
    """MediaSetViewItemWrapper"""

    media_set_view_item: MediaSetViewItem = pydantic.Field(alias=str("mediaSetViewItem"))  # type: ignore[literal-required]
    type: typing.Literal["mediaSetViewItem"] = "mediaSetViewItem"


MediaSetViewRid: typing_extensions.TypeAlias = core.RID
"""The Resource Identifier (RID) of a single View of a Media Set. A Media Set View is an independent collection of Media Items."""


MediaType: typing_extensions.TypeAlias = str
"""
The [media type](https://www.iana.org/assignments/media-types/media-types.xhtml) of the file or attachment.
Examples: `application/json`, `application/pdf`, `application/octet-stream`, `image/jpeg`
"""


NetworkEgressPolicyRid: typing_extensions.TypeAlias = core.RID
"""The Resource Identifier (RID) of a Network Egress Policy."""


class NullType(core.ModelBase):
    """NullType"""

    type: typing.Literal["null"] = "null"


class NumericOrNonNumericType(core.ModelBase):
    """
    The time series property can either contain either numeric or non-numeric data. This enables mixed sensor types
    where some sensor time series are numeric and others are categorical. A boolean property reference can be used
    to determine if the series is numeric or non-numeric. Without this property, the series type can be either
    numeric or non-numeric and must be inferred from the result of a time series query.
    """

    is_non_numeric_property_type_id: typing.Optional[str] = pydantic.Field(alias=str("isNonNumericPropertyTypeId"), default=None)  # type: ignore[literal-required]
    """
    The boolean property type ID specifying whether the series is numeric or non-numeric. If the value is true,
    the series is non-numeric.
    """

    type: typing.Literal["numericOrNonNumeric"] = "numericOrNonNumeric"


Operation: typing_extensions.TypeAlias = str
"""
An operation that can be performed on a resource. Operations are used to define the permissions that a Role has.
Operations are typically in the format `service:action`, where `service` is related to the type of resource and `action` is the action being performed.
"""


OperationScope: typing_extensions.TypeAlias = str
"""OperationScope"""


OrderByDirection: typing_extensions.TypeAlias = typing.Literal["ASC", "DESC"]
"""Specifies the ordering direction (can be either `ASC` or `DESC`)"""


OrganizationRid: typing_extensions.TypeAlias = core.RID
"""OrganizationRid"""


PageSize: typing_extensions.TypeAlias = int
"""The page size to use for the endpoint."""


PageToken: typing_extensions.TypeAlias = str
"""
The page token indicates where to start paging. This should be omitted from the first page's request.
To fetch the next page, clients should take the value from the `nextPageToken` field of the previous response
and use it to populate the `pageToken` field of the next request.
"""


PreviewMode: typing_extensions.TypeAlias = bool
"""Enables the use of preview functionality."""


PrincipalId: typing_extensions.TypeAlias = core.UUID
"""The ID of a Foundry Group or User."""


PrincipalType: typing_extensions.TypeAlias = typing.Literal["USER", "GROUP"]
"""PrincipalType"""


Realm: typing_extensions.TypeAlias = str
"""
Identifies which Realm a User or Group is a member of.
The `palantir-internal-realm` is used for Users or Groups that are created in Foundry by administrators and not associated with any SSO provider.
"""


ReleaseStatus: typing_extensions.TypeAlias = typing.Literal[
    "ACTIVE", "ENDORSED", "EXPERIMENTAL", "DEPRECATED"
]
"""The release status of the entity."""


class Role(core.ModelBase):
    """A set of permissions that can be assigned to a principal for a specific resource type."""

    id: RoleId
    role_set_id: RoleSetId = pydantic.Field(alias=str("roleSetId"))  # type: ignore[literal-required]
    name: str
    description: str
    is_default: bool = pydantic.Field(alias=str("isDefault"))  # type: ignore[literal-required]
    """Default roles are provided by Palantir and cannot be edited or modified by administrators."""

    type: RoleContext
    """The type of resource that is valid for this role."""

    operations: typing.List[Operation]
    """The operations that a principal can perform with this role on the assigned resource."""


class RoleAssignmentUpdate(core.ModelBase):
    """RoleAssignmentUpdate"""

    role_id: RoleId = pydantic.Field(alias=str("roleId"))  # type: ignore[literal-required]
    principal_id: PrincipalId = pydantic.Field(alias=str("principalId"))  # type: ignore[literal-required]


RoleContext: typing_extensions.TypeAlias = typing.Literal["ORGANIZATION"]
"""RoleContext"""


RoleId: typing_extensions.TypeAlias = str
"""
The unique ID for a Role. Roles are sets of permissions that grant different levels of access to resources.
The default roles in Foundry are: Owner, Editor, Viewer, and Discoverer. See more about 
[roles](https://palantir.com/docs/foundry/security/projects-and-roles#roles) in the user documentation.
"""


RoleSetId: typing_extensions.TypeAlias = str
"""RoleSetId"""


class ScenarioReferenceType(core.ModelBase):
    """ScenarioReferenceType"""

    type: typing.Literal["scenarioReference"] = "scenarioReference"


ScheduleRid: typing_extensions.TypeAlias = core.RID
"""The RID of a Schedule."""


SchemaFieldType: typing_extensions.TypeAlias = typing.Literal[
    "ARRAY",
    "BINARY",
    "BOOLEAN",
    "BYTE",
    "DATE",
    "DECIMAL",
    "DOUBLE",
    "FLOAT",
    "INTEGER",
    "LONG",
    "MAP",
    "SHORT",
    "STRING",
    "STRUCT",
    "TIMESTAMP",
]
"""The data type of a column in a dataset schema."""


class ShortType(core.ModelBase):
    """ShortType"""

    type: typing.Literal["short"] = "short"


SizeBytes: typing_extensions.TypeAlias = core.Long
"""The size of the file or attachment in bytes."""


class StreamSchema(core.ModelBase):
    """The schema for a Foundry stream. Records pushed to this stream must match this schema."""

    fields: typing.List[Field]
    key_field_names: typing.Optional[typing.List[FieldName]] = pydantic.Field(alias=str("keyFieldNames"), default=None)  # type: ignore[literal-required]
    """
    The names of the fields to be used as keys for partitioning records. These key fields are used to group
    all records with the same key into the same partition, to guarantee processing order of grouped records. These
    keys are not meant to uniquely identify records, and do not by themselves deduplicate records. To deduplicate
    records, provide a change data capture configuration for the schema.

    Key fields can only be of the following types:
    - Boolean
    - Byte
    - Date
    - Decimal
    - Integer
    - Long
    - Short
    - String
    - Timestamp

    For additional information on keys for Foundry streams, see the
    [streaming keys](https://palantir.com/docs/foundry/building-pipelines/streaming-keys/) user documentation.
    """

    change_data_capture: typing.Optional[ChangeDataCaptureConfiguration] = pydantic.Field(alias=str("changeDataCapture"), default=None)  # type: ignore[literal-required]


class StringType(core.ModelBase):
    """StringType"""

    type: typing.Literal["string"] = "string"


StructFieldName: typing_extensions.TypeAlias = str
"""The name of a field in a `Struct`."""


class StructFieldType(core.ModelBase):
    """StructFieldType"""

    sub_fields: typing.List[Field] = pydantic.Field(alias=str("subFields"))  # type: ignore[literal-required]
    type: typing.Literal["struct"] = "struct"


TableRid: typing_extensions.TypeAlias = core.RID
"""The Resource Identifier (RID) of a Table."""


TimeSeriesItemType: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["StringType", "DoubleType", "NumericOrNonNumericType"],
    pydantic.Field(discriminator="type"),
]
"""A union of the types supported by time series properties."""


TimeUnit: typing_extensions.TypeAlias = typing.Literal[
    "MILLISECONDS", "SECONDS", "MINUTES", "HOURS", "DAYS", "WEEKS", "MONTHS", "YEARS"
]
"""TimeUnit"""


class TimeseriesType(core.ModelBase):
    """TimeseriesType"""

    item_type: TimeSeriesItemType = pydantic.Field(alias=str("itemType"))  # type: ignore[literal-required]
    type: typing.Literal["timeseries"] = "timeseries"


class TimestampType(core.ModelBase):
    """TimestampType"""

    type: typing.Literal["timestamp"] = "timestamp"


TotalCount: typing_extensions.TypeAlias = core.Long
"""The total number of items across all pages."""


TraceParent: typing_extensions.TypeAlias = str
"""The W3C Trace Context `traceparent` header value used to propagate distributed tracing information for Foundry telemetry. See https://www.w3.org/TR/trace-context/#traceparent-header for more details. Note the 16 byte trace ID encoded in the header must be derived from a time based uuid to be used within Foundry."""


TraceState: typing_extensions.TypeAlias = str
"""The W3C Trace Context `tracestate` header value, which is used to propagate vendor specific distributed tracing information for Foundry telemetry. See https://www.w3.org/TR/trace-context/#tracestate-header for more details."""


class UnsupportedType(core.ModelBase):
    """UnsupportedType"""

    unsupported_type: str = pydantic.Field(alias=str("unsupportedType"))  # type: ignore[literal-required]
    params: typing.Dict[UnsupportedTypeParamKey, UnsupportedTypeParamValue]
    type: typing.Literal["unsupported"] = "unsupported"


UnsupportedTypeParamKey: typing_extensions.TypeAlias = str
"""UnsupportedTypeParamKey"""


UnsupportedTypeParamValue: typing_extensions.TypeAlias = str
"""UnsupportedTypeParamValue"""


UpdatedTime: typing_extensions.TypeAlias = core.AwareDatetime
"""The time at which the resource was most recently updated."""


UserId: typing_extensions.TypeAlias = core.UUID
"""A Foundry User ID."""


UserStatus: typing_extensions.TypeAlias = typing.Literal["ACTIVE", "DELETED"]
"""Present status of user."""


class VectorSimilarityFunction(core.ModelBase):
    """
    The vector similarity function to support approximate nearest neighbors search. Will result in an index
    specific for the function.
    """

    value: typing.Optional[VectorSimilarityFunctionValue] = None


VectorSimilarityFunctionValue: typing_extensions.TypeAlias = typing.Literal[
    "COSINE_SIMILARITY", "DOT_PRODUCT", "EUCLIDEAN_DISTANCE"
]
"""VectorSimilarityFunctionValue"""


class VectorType(core.ModelBase):
    """Represents a fixed size vector of floats. These can be used for vector similarity searches."""

    dimension: int
    """The dimension of the vector."""

    supports_search_with: typing.List[VectorSimilarityFunction] = pydantic.Field(alias=str("supportsSearchWith"))  # type: ignore[literal-required]
    embedding_model: typing.Optional[EmbeddingModel] = pydantic.Field(alias=str("embeddingModel"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["vector"] = "vector"


VersionId: typing_extensions.TypeAlias = core.UUID
"""The version identifier of a dataset schema."""


class VoidType(core.ModelBase):
    """VoidType"""

    type: typing.Literal["void"] = "void"


ZoneId: typing_extensions.TypeAlias = str
"""A string representation of a java.time.ZoneId"""


ChangeDataCaptureConfiguration: typing_extensions.TypeAlias = FullRowChangeDataCaptureConfiguration
"""
Configuration for utilizing the stream as a change data capture (CDC) dataset. To configure CDC on a stream, at
least one key needs to be provided.

For more information on CDC in
Foundry, see the [Change Data Capture](https://palantir.com/docs/foundry/data-integration/change-data-capture/) user documentation.
"""


CreatedBy: typing_extensions.TypeAlias = PrincipalId
"""The Foundry user who created this resource"""


Reference: typing_extensions.TypeAlias = MediaSetViewItemWrapper
"""A union of the types supported by media reference properties."""


UpdatedBy: typing_extensions.TypeAlias = UserId
"""The Foundry user who last updated this resource"""


core.resolve_forward_references_in_module(__name__)

__all__ = [
    "AnyType",
    "ArrayFieldType",
    "AttachmentType",
    "Attribution",
    "BinaryType",
    "BooleanType",
    "BranchMetadata",
    "BranchName",
    "BuildRid",
    "ByteType",
    "ChangeDataCaptureConfiguration",
    "CheckReportRid",
    "CheckRid",
    "CipherTextType",
    "Color",
    "ColumnName",
    "ComputeSeconds",
    "ContentLength",
    "ContentType",
    "CreatedBy",
    "CreatedTime",
    "CustomMetadata",
    "DatasetFieldSchema",
    "DatasetRid",
    "DatasetSchema",
    "DateType",
    "DecimalType",
    "DisplayName",
    "Distance",
    "DistanceUnit",
    "DoubleType",
    "Duration",
    "DurationSeconds",
    "EmbeddingModel",
    "EnrollmentRid",
    "Field",
    "FieldDataType",
    "FieldName",
    "FieldSchema",
    "FilePath",
    "Filename",
    "FilterBinaryType",
    "FilterBooleanType",
    "FilterDateTimeType",
    "FilterDateType",
    "FilterDoubleType",
    "FilterEnumType",
    "FilterFloatType",
    "FilterIntegerType",
    "FilterLongType",
    "FilterRidType",
    "FilterStringType",
    "FilterType",
    "FilterUuidType",
    "FloatType",
    "FolderRid",
    "FoundryBranch",
    "FoundryLiveDeployment",
    "FullRowChangeDataCaptureConfiguration",
    "GeoPointType",
    "GeoShapeType",
    "GeohashType",
    "GeotimeSeriesReferenceType",
    "GroupId",
    "GroupName",
    "GroupRid",
    "IncludeComputeUsage",
    "IntegerType",
    "JobRid",
    "LmsEmbeddingModel",
    "LmsEmbeddingModelValue",
    "LongType",
    "MapFieldType",
    "MarkingId",
    "MarkingType",
    "MarkingTypeValue",
    "MediaItemPath",
    "MediaItemReadToken",
    "MediaItemRid",
    "MediaReference",
    "MediaReferenceType",
    "MediaSetRid",
    "MediaSetViewItem",
    "MediaSetViewItemWrapper",
    "MediaSetViewRid",
    "MediaType",
    "NetworkEgressPolicyRid",
    "NullType",
    "NumericOrNonNumericType",
    "Operation",
    "OperationScope",
    "OrderByDirection",
    "OrganizationRid",
    "PageSize",
    "PageToken",
    "PreviewMode",
    "PrincipalId",
    "PrincipalType",
    "Realm",
    "Reference",
    "ReleaseStatus",
    "Role",
    "RoleAssignmentUpdate",
    "RoleContext",
    "RoleId",
    "RoleSetId",
    "ScenarioReferenceType",
    "ScheduleRid",
    "SchemaFieldType",
    "ShortType",
    "SizeBytes",
    "StreamSchema",
    "StringType",
    "StructFieldName",
    "StructFieldType",
    "TableRid",
    "TimeSeriesItemType",
    "TimeUnit",
    "TimeseriesType",
    "TimestampType",
    "TotalCount",
    "TraceParent",
    "TraceState",
    "UnsupportedType",
    "UnsupportedTypeParamKey",
    "UnsupportedTypeParamValue",
    "UpdatedBy",
    "UpdatedTime",
    "UserId",
    "UserStatus",
    "VectorSimilarityFunction",
    "VectorSimilarityFunctionValue",
    "VectorType",
    "VersionId",
    "VoidType",
    "ZoneId",
]
