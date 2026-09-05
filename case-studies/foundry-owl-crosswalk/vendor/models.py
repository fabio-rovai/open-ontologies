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
from datetime import date

import annotated_types
import pydantic
import typing_extensions

from foundry_sdk import _core as core
from foundry_sdk.v2.core import models as core_models
from foundry_sdk.v2.datasets import models as datasets_models
from foundry_sdk.v2.geo import models as geo_models


class AbsoluteTimeRange(core.ModelBase):
    """ISO 8601 timestamps forming a range for a time series query. Start is inclusive and end is exclusive."""

    start_time: typing.Optional[core.AwareDatetime] = pydantic.Field(alias=str("startTime"), default=None)  # type: ignore[literal-required]
    end_time: typing.Optional[core.AwareDatetime] = pydantic.Field(alias=str("endTime"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["absolute"] = "absolute"


class AbsoluteValuePropertyExpression(core.ModelBase):
    """Calculates absolute value of a numeric value."""

    property: DerivedPropertyDefinition
    type: typing.Literal["absoluteValue"] = "absoluteValue"


ActionExecutionTime: typing_extensions.TypeAlias = core.AwareDatetime
"""An ISO 8601 timestamp."""


ActionLogicRule: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "ModifyInterfaceLogicRule",
        "CreateOrModifyObjectLogicRule",
        "ModifyObjectLogicRule",
        "DeleteLinkLogicRule",
        "CreateObjectLogicRule",
        "CreateLinkLogicRule",
        "BatchedFunctionLogicRule",
        "CreateOrModifyObjectLogicRuleV2",
        "DeleteInterfaceLinkLogicRule",
        "DeleteObjectLogicRule",
        "FunctionLogicRule",
        "CreateInterfaceLinkLogicRule",
        "CreateInterfaceLogicRule",
        "ApplyScenarioLogicRule",
    ],
    pydantic.Field(discriminator="type"),
]
"""A detailed operation for an Action"""


class ActionParameterArrayType(core.ModelBase):
    """ActionParameterArrayType"""

    sub_type: ActionParameterType = pydantic.Field(alias=str("subType"))  # type: ignore[literal-required]
    type: typing.Literal["array"] = "array"


ActionParameterRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of an action parameter, useful for interacting with other Foundry APIs."""


ActionParameterType: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        core_models.DateType,
        "OntologyInterfaceObjectType",
        "OntologyStructType",
        core_models.StringType,
        core_models.DoubleType,
        core_models.IntegerType,
        core_models.GeoShapeType,
        core_models.LongType,
        "OntologyObjectTypeReferenceType",
        core_models.BooleanType,
        core_models.MarkingType,
        core_models.ScenarioReferenceType,
        core_models.AttachmentType,
        core_models.MediaReferenceType,
        "ActionParameterArrayType",
        "OntologyObjectSetType",
        core_models.GeohashType,
        core_models.VectorType,
        core_models.DecimalType,
        "OntologyObjectType",
        core_models.TimestampType,
    ],
    pydantic.Field(discriminator="type"),
]
"""A union of all the types supported by Ontology Action parameters."""


class ActionParameterV2(core.ModelBase):
    """Details about a parameter of an action."""

    display_name: core_models.DisplayName = pydantic.Field(alias=str("displayName"))  # type: ignore[literal-required]
    description: typing.Optional[str] = None
    data_type: ActionParameterType = pydantic.Field(alias=str("dataType"))  # type: ignore[literal-required]
    required: bool
    type_classes: typing.Optional[typing.List[TypeClass]] = pydantic.Field(alias=str("typeClasses"), default=None)  # type: ignore[literal-required]
    validation: typing.Optional[ActionParameterValidation] = None


class ActionParameterValidation(core.ModelBase):
    """Validation metadata surfaced for a parameter."""

    default_validation: ActionParameterValidationBlock = pydantic.Field(alias=str("defaultValidation"))  # type: ignore[literal-required]


class ActionParameterValidationBlock(core.ModelBase):
    """Validation constraints for a parameter."""

    allowed_values: typing.Optional[ParameterAllowedValues] = pydantic.Field(alias=str("allowedValues"), default=None)  # type: ignore[literal-required]
    array_size: typing.Optional[ParameterArraySize] = pydantic.Field(alias=str("arraySize"), default=None)  # type: ignore[literal-required]


ActionResults: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["ObjectEdits", "ObjectTypeEdits"], pydantic.Field(discriminator="type")
]
"""ActionResults"""


ActionRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier for an action."""


ActionSectionRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of an action section, useful for interacting with other Foundry APIs."""


ActionTypeApiName: typing_extensions.TypeAlias = str
"""
The name of the action type in the API. To find the API name for your Action Type, use the `List action types`
endpoint or check the **Ontology Manager**.
"""


class ActionTypeApiNameActionTypesQueryV2(core.ModelBase):
    """Returns action types with an api name matching the given string predicate."""

    value: FullTextStringPredicateV2
    type: typing.Literal["actionTypeApiName"] = "actionTypeApiName"


class ActionTypeDescriptionActionTypesQueryV2(core.ModelBase):
    """Returns action types with a description matching the given string predicate."""

    value: FullTextStringPredicateV2
    type: typing.Literal["actionTypeDescription"] = "actionTypeDescription"


class ActionTypeDisplayNameActionTypesQueryV2(core.ModelBase):
    """Returns action types with a display name matching the given string predicate."""

    value: FullTextStringPredicateV2
    type: typing.Literal["actionTypeDisplayName"] = "actionTypeDisplayName"


class ActionTypeFullMetadata(core.ModelBase):
    """Returns the full metadata for an Action type in the Ontology."""

    action_type: ActionTypeV2 = pydantic.Field(alias=str("actionType"))  # type: ignore[literal-required]
    full_logic_rules: typing.List[ActionLogicRule] = pydantic.Field(alias=str("fullLogicRules"))  # type: ignore[literal-required]


ActionTypeFuzziness: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["FuzzinessAuto", "FuzzinessOff"], pydantic.Field(discriminator="type")
]
"""
Fuzziness setting applied to `contains` full-text string predicates in the search query. If not provided,
`auto` is used.
"""


ActionTypeLogicRuleTypeFilter: typing_extensions.TypeAlias = typing.Literal[
    "ADD_OBJECT",
    "MODIFY_OBJECT",
    "DELETE_OBJECT",
    "ADD_LINK",
    "DELETE_LINK",
    "FUNCTION",
    "BATCHED_FUNCTION",
    "ADD_OR_MODIFY_OBJECT",
    "ADD_OR_MODIFY_OBJECT_V2",
]
"""Filter action types by the type of logic rule they contain."""


ActionTypePermissionModelFilter: typing_extensions.TypeAlias = typing.Literal[
    "DATASOURCE_DERIVED_PERMISSIONS", "ONTOLOGY_ROLES", "COMPASS_PROJECT"
]
"""Filter action types by permission model."""


ActionTypeRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of an action type, useful for interacting with other Foundry APIs."""


class ActionTypeRidActionTypesQueryV2(core.ModelBase):
    """Returns action types with the given rid."""

    value: ActionTypeRid
    type: typing.Literal["actionTypeRid"] = "actionTypeRid"


ActionTypeSearchJsonQueryV2: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "WebhookRidActionTypesQueryV2",
        "HasActionLogActionTypesQueryV2",
        "ActionTypeApiNameActionTypesQueryV2",
        "OrActionTypesQueryV2",
        "PermissionModelActionTypesQueryV2",
        "HasWebhookActionTypesQueryV2",
        "ActionTypeDisplayNameActionTypesQueryV2",
        "AffectedLinkTypeRidActionTypesQueryV2",
        "ActionTypeDescriptionActionTypesQueryV2",
        "SectionRidActionTypesQueryV2",
        "ParameterRidActionTypesQueryV2",
        "ParameterNameActionTypesQueryV2",
        "HasNotificationActionTypesQueryV2",
        "FunctionRidActionTypesQueryV2",
        "InputObjectTypeRidActionTypesQueryV2",
        "RevertActionEnabledActionTypesQueryV2",
        "LogicRuleActionTypesQueryV2",
        "TypeClassesActionTypesQueryV2",
        "AffectedInterfaceTypeRidActionTypesQueryV2",
        "AndActionTypesQueryV2",
        "ActionTypeRidActionTypesQueryV2",
        "AffectedObjectTypeRidActionTypesQueryV2",
        "StatusActionTypesQueryV2",
    ],
    pydantic.Field(discriminator="type"),
]
"""Represents the search query for an action type search. Supports filters for various action type features."""


ActionTypeSortByV2: typing_extensions.TypeAlias = typing.Literal["actionTypeDisplayName"]
"""Specifies the field to sort action types by."""


ActionTypeStatusFilter: typing_extensions.TypeAlias = typing.Literal[
    "EXPERIMENTAL", "ACTIVE", "DEPRECATED", "EXAMPLE"
]
"""Filter action types by status."""


class ActionTypeV2(core.ModelBase):
    """Represents an action type in the Ontology."""

    api_name: ActionTypeApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    description: typing.Optional[str] = None
    display_name: typing.Optional[core_models.DisplayName] = pydantic.Field(alias=str("displayName"), default=None)  # type: ignore[literal-required]
    status: core_models.ReleaseStatus
    parameters: typing.Dict[ParameterId, ActionParameterV2]
    rid: ActionTypeRid
    operations: typing.List[LogicRule]
    tool_description: typing.Optional[str] = pydantic.Field(alias=str("toolDescription"), default=None)  # type: ignore[literal-required]
    """Optional description intended for tool use contexts, such as AI agents."""


class ActivePropertyTypeStatus(core.ModelBase):
    """
    This status indicates that the PropertyType will not change on short notice and should thus be safe to use in
    user facing workflows.
    """

    type: typing.Literal["active"] = "active"


class AddLink(core.ModelBase):
    """AddLink"""

    link_type_api_name_ato_b: LinkTypeApiName = pydantic.Field(alias=str("linkTypeApiNameAtoB"))  # type: ignore[literal-required]
    link_type_api_name_bto_a: LinkTypeApiName = pydantic.Field(alias=str("linkTypeApiNameBtoA"))  # type: ignore[literal-required]
    a_side_object: LinkSideObject = pydantic.Field(alias=str("aSideObject"))  # type: ignore[literal-required]
    b_side_object: LinkSideObject = pydantic.Field(alias=str("bSideObject"))  # type: ignore[literal-required]
    type: typing.Literal["addLink"] = "addLink"


class AddLinkEdit(core.ModelBase):
    """AddLinkEdit"""

    object_type: ObjectTypeApiName = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]
    primary_key: PrimaryKeyValue = pydantic.Field(alias=str("primaryKey"))  # type: ignore[literal-required]
    link_type: LinkTypeApiName = pydantic.Field(alias=str("linkType"))  # type: ignore[literal-required]
    linked_object_primary_key: PrimaryKeyValue = pydantic.Field(alias=str("linkedObjectPrimaryKey"))  # type: ignore[literal-required]
    type: typing.Literal["addLink"] = "addLink"


class AddObject(core.ModelBase):
    """AddObject"""

    primary_key: PropertyValue = pydantic.Field(alias=str("primaryKey"))  # type: ignore[literal-required]
    object_type: ObjectTypeApiName = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]
    type: typing.Literal["addObject"] = "addObject"


class AddObjectEdit(core.ModelBase):
    """AddObjectEdit"""

    object_type: ObjectTypeApiName = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]
    properties: typing.Dict[PropertyApiName, typing.Optional[DataValue]]
    type: typing.Literal["addObject"] = "addObject"


class AddPropertyExpression(core.ModelBase):
    """Adds two or more numeric values."""

    properties: typing.List[DerivedPropertyDefinition]
    type: typing.Literal["add"] = "add"


class AffectedInterfaceTypeRidActionTypesQueryV2(core.ModelBase):
    """Returns action types which edit the interface type with the given rid."""

    value: InterfaceTypeRid
    type: typing.Literal["affectedInterfaceTypeRid"] = "affectedInterfaceTypeRid"


class AffectedLinkTypeRidActionTypesQueryV2(core.ModelBase):
    """Returns action types which edit the link type with the given rid."""

    value: LinkTypeRid
    type: typing.Literal["affectedLinkTypeRid"] = "affectedLinkTypeRid"


class AffectedObjectTypeRidActionTypesQueryV2(core.ModelBase):
    """Returns action types which edit the object type with the given rid."""

    value: ObjectTypeRid
    type: typing.Literal["affectedObjectTypeRid"] = "affectedObjectTypeRid"


class Affix(core.ModelBase):
    """Affix"""

    prefix: typing.Optional[PropertyTypeReferenceOrStringConstant] = None
    postfix: typing.Optional[PropertyTypeReferenceOrStringConstant] = None


class AggregateObjectSetRequestV2(core.ModelBase):
    """AggregateObjectSetRequestV2"""

    aggregation: typing.List[AggregationV2]
    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    group_by: typing.List[AggregationGroupByV2] = pydantic.Field(alias=str("groupBy"))  # type: ignore[literal-required]
    accuracy: typing.Optional[AggregationAccuracyRequest] = None
    include_compute_usage: typing.Optional[core_models.IncludeComputeUsage] = pydantic.Field(alias=str("includeComputeUsage"), default=None)  # type: ignore[literal-required]


class AggregateObjectsRequestV2(core.ModelBase):
    """AggregateObjectsRequestV2"""

    aggregation: typing.List[AggregationV2]
    where: typing.Optional[SearchJsonQueryV2] = None
    group_by: typing.List[AggregationGroupByV2] = pydantic.Field(alias=str("groupBy"))  # type: ignore[literal-required]
    accuracy: typing.Optional[AggregationAccuracyRequest] = None


class AggregateObjectsResponseItemV2(core.ModelBase):
    """AggregateObjectsResponseItemV2"""

    group: typing.Dict[AggregationGroupKeyV2, typing.Optional[AggregationGroupValueV2]]
    metrics: typing.List[AggregationMetricResultV2]


class AggregateObjectsResponseV2(core.ModelBase):
    """AggregateObjectsResponseV2"""

    excluded_items: typing.Optional[int] = pydantic.Field(alias=str("excludedItems"), default=None)  # type: ignore[literal-required]
    accuracy: AggregationAccuracy
    data: typing.List[AggregateObjectsResponseItemV2]
    compute_usage: typing.Optional[core_models.ComputeSeconds] = pydantic.Field(alias=str("computeUsage"), default=None)  # type: ignore[literal-required]


class AggregateTimeSeries(core.ModelBase):
    """AggregateTimeSeries"""

    method: TimeSeriesAggregationMethod
    strategy: TimeSeriesAggregationStrategy


AggregationAccuracy: typing_extensions.TypeAlias = typing.Literal["ACCURATE", "APPROXIMATE"]
"""AggregationAccuracy"""


AggregationAccuracyRequest: typing_extensions.TypeAlias = typing.Literal[
    "REQUIRE_ACCURATE", "ALLOW_APPROXIMATE"
]
"""
Specifies the accuracy requirement for aggregation results.

- `REQUIRE_ACCURATE`: Only return results if they are guaranteed to be accurate. If accuracy cannot be
  guaranteed (e.g., due to a low `maxGroupCount` relative to distinct values), the request will fail
  with an `AggregationAccuracyNotSupported` error.
- `ALLOW_APPROXIMATE`: Allow approximate results when exact computation is not feasible. This is the
  default behavior if not specified.
"""


class AggregationDurationGroupingV2(core.ModelBase):
    """
    Divides objects into groups according to an interval. Note that this grouping applies only on date and timestamp types.
    When grouping by `YEARS`, `QUARTERS`, `MONTHS`, or `WEEKS`, the `value` must be set to `1`.
    Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: int
    unit: TimeUnit
    type: typing.Literal["duration"] = "duration"


class AggregationExactGroupingV2(core.ModelBase):
    """
    Divides objects into groups according to an exact value.
    Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    max_group_count: typing.Optional[int] = pydantic.Field(alias=str("maxGroupCount"), default=None)  # type: ignore[literal-required]
    """
    The maximum number of groups to return. If omitted, defaults to 10,000.

    The server allocates resources based on the specified `maxGroupCount`. When the number of distinct
    values in your data is within this limit, results are accurate and the top N values are returned
    correctly. When distinct values exceed what the allocated resources can handle, results may become
    approximate.

    If you need accurate results with high-cardinality properties, set `maxGroupCount` high enough to
    cover your distinct values. Items exceeding the limit are excluded from results and counted in
    `excludedItems`. The response `accuracy` field indicates whether the results are `ACCURATE` or
    `APPROXIMATE`.
    """

    default_value: typing.Optional[str] = pydantic.Field(alias=str("defaultValue"), default=None)  # type: ignore[literal-required]
    """
    Includes a group with the specified default value that includes all objects where the specified field's value is null.
    Cannot be used with includeNullValues.
    """

    include_null_values: typing.Optional[bool] = pydantic.Field(alias=str("includeNullValues"), default=None)  # type: ignore[literal-required]
    """
    Includes a group with a null value that includes all objects where the specified field's value is null.
    Cannot be used with defaultValue or orderBy clauses on the aggregation.
    """

    type: typing.Literal["exact"] = "exact"


class AggregationFixedWidthGroupingV2(core.ModelBase):
    """
    Divides objects into groups with the specified width.
    Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    fixed_width: int = pydantic.Field(alias=str("fixedWidth"))  # type: ignore[literal-required]
    type: typing.Literal["fixedWidth"] = "fixedWidth"


AggregationGroupByV2: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "AggregationDurationGroupingV2",
        "AggregationFixedWidthGroupingV2",
        "AggregationRangesGroupingV2",
        "AggregationExactGroupingV2",
    ],
    pydantic.Field(discriminator="type"),
]
"""Specifies a grouping for aggregation results."""


AggregationGroupKeyV2: typing_extensions.TypeAlias = str
"""AggregationGroupKeyV2"""


AggregationGroupValueV2: typing_extensions.TypeAlias = typing.Any
"""AggregationGroupValueV2"""


AggregationMetricName: typing_extensions.TypeAlias = str
"""A user-specified alias for an aggregation metric name."""


class AggregationMetricResultV2(core.ModelBase):
    """AggregationMetricResultV2"""

    name: str
    value: typing.Optional[typing.Any] = None
    """
    The value of the metric. This will be a double in the case of
    a numeric metric, or a date string in the case of a date metric.
    """


class AggregationRangeV2(core.ModelBase):
    """Specifies a range from an inclusive start value to an exclusive end value."""

    start_value: typing.Any = pydantic.Field(alias=str("startValue"))  # type: ignore[literal-required]
    """Inclusive start."""

    end_value: typing.Any = pydantic.Field(alias=str("endValue"))  # type: ignore[literal-required]
    """Exclusive end."""


class AggregationRangesGroupingV2(core.ModelBase):
    """
    Divides objects into groups according to specified ranges.
    Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    ranges: typing.List[AggregationRangeV2]
    type: typing.Literal["ranges"] = "ranges"


AggregationV2: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "ApproximateDistinctAggregationV2",
        "MinAggregationV2",
        "AvgAggregationV2",
        "MaxAggregationV2",
        "ApproximatePercentileAggregationV2",
        "CountAggregationV2",
        "SumAggregationV2",
        "ExactDistinctAggregationV2",
    ],
    pydantic.Field(discriminator="type"),
]
"""Specifies an aggregation function."""


class AllOfRule(core.ModelBase):
    """Matches intervals satisfying all the rules in the query"""

    rules: typing.List[IntervalQueryRule]
    max_gaps: typing.Optional[int] = pydantic.Field(alias=str("maxGaps"), default=None)  # type: ignore[literal-required]
    """The maximum gaps between the intervals produced by the sub-rules. If not set, then gaps are not considered."""

    ordered: bool
    """If true, the matched intervals must occur in order."""

    type: typing.Literal["allOf"] = "allOf"


class AndActionTypesQueryV2(core.ModelBase):
    """Returns action types where every query is satisfied. An empty list matches all action types."""

    value: typing.List[ActionTypeSearchJsonQueryV2]
    type: typing.Literal["and"] = "and"


class AndQueryV2(core.ModelBase):
    """Returns objects where every query is satisfied."""

    value: typing.List[SearchJsonQueryV2]
    type: typing.Literal["and"] = "and"


class AnyOfRule(core.ModelBase):
    """Matches intervals satisfying any of the rules in the query"""

    rules: typing.List[IntervalQueryRule]
    type: typing.Literal["anyOf"] = "anyOf"


ApplyActionMode: typing_extensions.TypeAlias = typing.Literal[
    "VALIDATE_ONLY", "VALIDATE_AND_EXECUTE"
]
"""If not specified, defaults to `VALIDATE_AND_EXECUTE`."""


class ApplyActionOverrides(core.ModelBase):
    """ApplyActionOverrides"""

    unique_identifier_link_id_values: typing.Dict[UniqueIdentifierLinkId, UniqueIdentifierValue] = pydantic.Field(alias=str("uniqueIdentifierLinkIdValues"))  # type: ignore[literal-required]
    action_execution_time: typing.Optional[ActionExecutionTime] = pydantic.Field(alias=str("actionExecutionTime"), default=None)  # type: ignore[literal-required]


class ApplyActionRequestOptions(core.ModelBase):
    """ApplyActionRequestOptions"""

    mode: typing.Optional[ApplyActionMode] = None
    return_edits: typing.Optional[ReturnEditsMode] = pydantic.Field(alias=str("returnEdits"), default=None)  # type: ignore[literal-required]


class ApplyActionRequestV2(core.ModelBase):
    """ApplyActionRequestV2"""

    options: typing.Optional[ApplyActionRequestOptions] = None
    parameters: typing.Dict[ParameterId, typing.Optional[DataValue]]


class ApplyActionWithOverridesRequest(core.ModelBase):
    """ApplyActionWithOverridesRequest"""

    request: ApplyActionRequestV2
    overrides: ApplyActionOverrides


class ApplyReducersAndExtractMainValueLoadLevel(core.ModelBase):
    """Performs both apply reducers and extract main value to return the reduced main value."""

    type: typing.Literal["applyReducersAndExtractMainValue"] = "applyReducersAndExtractMainValue"


class ApplyReducersLoadLevel(core.ModelBase):
    """Returns a single value of an array as configured in the ontology."""

    type: typing.Literal["applyReducers"] = "applyReducers"


class ApplyScenarioLogicRule(core.ModelBase):
    """
    An Action rule that merges the edits accumulated on a referenced Scenario into the ontology data context
    where the Action is applied. If the Action is applied against another Scenario, the edits are merged into
    that target Scenario.

    The scenario is supplied through the parameter identified by `scenarioParameter`, whose value type is
    `scenarioReference`. The affected object types and link types are explicitly enumerated in the scope.
    """

    scenario_parameter: ParameterId = pydantic.Field(alias=str("scenarioParameter"))  # type: ignore[literal-required]
    object_type_api_names: typing.List[ObjectTypeApiName] = pydantic.Field(alias=str("objectTypeApiNames"))  # type: ignore[literal-required]
    link_types: typing.List[ObjectTypeLinkTypeApiNameMapping] = pydantic.Field(alias=str("linkTypes"))  # type: ignore[literal-required]
    type: typing.Literal["applyScenario"] = "applyScenario"


class ApplyScenarioRule(core.ModelBase):
    """
    An Action rule that applies the edits accumulated on a referenced Scenario onto the ontology data context
    where the Action is applied. If the Action is applied in the context of main ontology data, the edits are
    applied there. If the Action is applied in the context of another Scenario, the edits are applied in that
    other Scenario.

    The scenario is supplied through the parameter identified by `scenarioParameter` of type
    `scenarioReference`. The affected object types and link types are explicitly enumerated in the scope.
    """

    scenario_parameter: ParameterId = pydantic.Field(alias=str("scenarioParameter"))  # type: ignore[literal-required]
    object_type_api_names: typing.List[ObjectTypeApiName] = pydantic.Field(alias=str("objectTypeApiNames"))  # type: ignore[literal-required]
    link_types: typing.List[ObjectTypeLinkTypeApiNameMapping] = pydantic.Field(alias=str("linkTypes"))  # type: ignore[literal-required]
    type: typing.Literal["applyScenario"] = "applyScenario"


class ApproximateDistinctAggregationV2(core.ModelBase):
    """
    Computes an approximate number of distinct values for the provided field.
    Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    name: typing.Optional[AggregationMetricName] = None
    direction: typing.Optional[OrderByDirection] = None
    type: typing.Literal["approximateDistinct"] = "approximateDistinct"


class ApproximatePercentileAggregationV2(core.ModelBase):
    """
    Computes the approximate percentile value for the provided field. Requires Object Storage V2.
    Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    name: typing.Optional[AggregationMetricName] = None
    approximate_percentile: float = pydantic.Field(alias=str("approximatePercentile"))  # type: ignore[literal-required]
    direction: typing.Optional[OrderByDirection] = None
    type: typing.Literal["approximatePercentile"] = "approximatePercentile"


class Arg(core.ModelBase):
    """Arg"""

    name: str
    value: str


class ArrayConstraint(core.ModelBase):
    """ArrayConstraint"""

    minimum_size: typing.Optional[int] = pydantic.Field(alias=str("minimumSize"), default=None)  # type: ignore[literal-required]
    maximum_size: typing.Optional[int] = pydantic.Field(alias=str("maximumSize"), default=None)  # type: ignore[literal-required]
    unique_values: bool = pydantic.Field(alias=str("uniqueValues"))  # type: ignore[literal-required]
    value_constraint: typing.Optional[ValueTypeConstraint] = pydantic.Field(alias=str("valueConstraint"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["array"] = "array"


class ArrayEvaluatedConstraint(core.ModelBase):
    """Evaluated constraints of array parameters that support per-entry constraint evaluations."""

    entries: typing.List[ArrayEntryEvaluatedConstraint]
    type: typing.Literal["array"] = "array"


class ArraySizeConstraint(core.ModelBase):
    """The parameter expects an array of values and the size of the array must fall within the defined range."""

    lt: typing.Optional[typing.Any] = None
    """Less than"""

    lte: typing.Optional[typing.Any] = None
    """Less than or equal"""

    gt: typing.Optional[typing.Any] = None
    """Greater than"""

    gte: typing.Optional[typing.Any] = None
    """Greater than or equal"""

    type: typing.Literal["arraySize"] = "arraySize"


ArtifactRepositoryRid: typing_extensions.TypeAlias = core.RID
"""ArtifactRepositoryRid"""


class AttachmentAllowedValues(core.ModelBase):
    """The parameter value (an attachment rid) must reference an attachment within the configured size limit."""

    max_size_bytes: typing.Optional[core_models.SizeBytes] = pydantic.Field(alias=str("maxSizeBytes"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["attachment"] = "attachment"


AttachmentMetadataResponse: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["AttachmentV2", "ListAttachmentsResponseV2"], pydantic.Field(discriminator="type")
]
"""The attachment metadata response"""


AttachmentRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of an attachment."""


class AttachmentV2(core.ModelBase):
    """The representation of an attachment."""

    rid: AttachmentRid
    filename: core_models.Filename
    size_bytes: core_models.SizeBytes = pydantic.Field(alias=str("sizeBytes"))  # type: ignore[literal-required]
    media_type: core_models.MediaType = pydantic.Field(alias=str("mediaType"))  # type: ignore[literal-required]
    type: typing.Literal["single"] = "single"


class AvgAggregationV2(core.ModelBase):
    """
    Computes the average value for the provided field.
    Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    name: typing.Optional[AggregationMetricName] = None
    direction: typing.Optional[OrderByDirection] = None
    type: typing.Literal["avg"] = "avg"


BatchActionObjectEdit: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["ModifyObject", "AddObject", "AddLink"], pydantic.Field(discriminator="type")
]
"""BatchActionObjectEdit"""


class BatchActionObjectEdits(core.ModelBase):
    """BatchActionObjectEdits"""

    edits: typing.List[BatchActionObjectEdit]
    added_object_count: int = pydantic.Field(alias=str("addedObjectCount"))  # type: ignore[literal-required]
    modified_objects_count: int = pydantic.Field(alias=str("modifiedObjectsCount"))  # type: ignore[literal-required]
    deleted_objects_count: int = pydantic.Field(alias=str("deletedObjectsCount"))  # type: ignore[literal-required]
    added_links_count: int = pydantic.Field(alias=str("addedLinksCount"))  # type: ignore[literal-required]
    deleted_links_count: int = pydantic.Field(alias=str("deletedLinksCount"))  # type: ignore[literal-required]
    type: typing.Literal["edits"] = "edits"


BatchActionResults: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["BatchActionObjectEdits", "ObjectTypeEdits"], pydantic.Field(discriminator="type")
]
"""BatchActionResults"""


class BatchApplyActionRequestItem(core.ModelBase):
    """BatchApplyActionRequestItem"""

    parameters: typing.Dict[ParameterId, typing.Optional[DataValue]]


class BatchApplyActionRequestItemWithOverrides(core.ModelBase):
    """BatchApplyActionRequestItemWithOverrides"""

    parameters: typing.Dict[ParameterId, typing.Optional[DataValue]]
    overrides: typing.Optional[ApplyActionOverrides] = None


class BatchApplyActionRequestOptions(core.ModelBase):
    """BatchApplyActionRequestOptions"""

    return_edits: typing.Optional[BatchReturnEditsMode] = pydantic.Field(alias=str("returnEdits"), default=None)  # type: ignore[literal-required]


class BatchApplyActionRequestV2(core.ModelBase):
    """BatchApplyActionRequestV2"""

    options: typing.Optional[BatchApplyActionRequestOptions] = None
    requests: typing.List[BatchApplyActionRequestItem]


class BatchApplyActionResponseV2(core.ModelBase):
    """BatchApplyActionResponseV2"""

    edits: typing.Optional[BatchActionResults] = None


class BatchApplyActionWithOverridesRequest(core.ModelBase):
    """BatchApplyActionWithOverridesRequest"""

    options: typing.Optional[BatchApplyActionRequestOptions] = None
    requests: typing.List[BatchApplyActionRequestItemWithOverrides]


BatchReturnEditsMode: typing_extensions.TypeAlias = typing.Literal["ALL", "NONE"]
"""If not specified, defaults to `NONE`."""


class BatchedFunctionLogicRule(core.ModelBase):
    """BatchedFunctionLogicRule"""

    object_set_rid_input_name: FunctionParameterName = pydantic.Field(alias=str("objectSetRidInputName"))  # type: ignore[literal-required]
    function_rule: FunctionLogicRule = pydantic.Field(alias=str("functionRule"))  # type: ignore[literal-required]
    type: typing.Literal["batchedFunction"] = "batchedFunction"


class BlueprintIcon(core.ModelBase):
    """BlueprintIcon"""

    color: str
    """A hexadecimal color code."""

    name: str
    """
    The [name](https://blueprintjs.com/docs/#icons/icons-list) of the Blueprint icon. 
    Used to specify the Blueprint icon to represent the object type in a React app.
    """

    type: typing.Literal["blueprint"] = "blueprint"


class BooleanValue(core.ModelBase):
    """BooleanValue"""

    value: bool
    type: typing.Literal["booleanValue"] = "booleanValue"


class BoundingBoxValue(core.ModelBase):
    """The top left and bottom right coordinate points that make up the bounding box."""

    top_left: WithinBoundingBoxPoint = pydantic.Field(alias=str("topLeft"))  # type: ignore[literal-required]
    bottom_right: WithinBoundingBoxPoint = pydantic.Field(alias=str("bottomRight"))  # type: ignore[literal-required]
    type: typing.Literal["envelope"] = "envelope"


class CenterPoint(core.ModelBase):
    """The coordinate point to use as the center of the distance query."""

    center: CenterPointTypes
    distance: core_models.Distance


CipherChannelStrategy: typing_extensions.TypeAlias = typing.Literal[
    "PREFER_EXISTING", "PREFER_DEFAULT", "EXISTING_ONLY", "DEFAULT_ONLY"
]
"""
Controls which Cipher Channel is used when encrypting a value. If not specified, defaults to `PREFER_EXISTING`.

- `PREFER_EXISTING`: use the Cipher Channel parsed from the existing ciphertext value; fall back to the default channel configured in ontology metadata.
- `PREFER_DEFAULT`: use the default channel configured in ontology metadata; fall back to the channel parsed from the existing ciphertext value.
- `EXISTING_ONLY`: use the channel parsed from the existing ciphertext value only; error if the value is not already encrypted.
- `DEFAULT_ONLY`: use the default channel configured in ontology metadata only; error if none is configured.
"""


CipherText: typing_extensions.TypeAlias = str
"""
A value encrypted with [Cipher](https://palantir.com/docs/foundry/cipher/overview/), stored in its envelope form which encodes
the Cipher Channel used to encrypt it (for example `CIPHER::{cipherChannelRid}::<ciphertext>::CIPHER`).
"""


ColumnName: typing_extensions.TypeAlias = str
"""The name of a column in a tabular datasource."""


class ColumnPropertyMapping(core.ModelBase):
    """A property bound to a single column in the backing datasource."""

    column: ColumnName
    type: typing.Literal["column"] = "column"


ConjunctiveMarkingSummary: typing_extensions.TypeAlias = typing.List["MarkingId"]
"""
The conjunctive set of markings required to access the property value. 
All markings from a conjunctive set must be met for access.
"""


ContainerConjunctiveMarkingSummary: typing_extensions.TypeAlias = typing.List["MarkingId"]
"""
The conjunctive set of markings for the container of this property value,
such as the project of a dataset. These markings may differ from the marking
on the actual property value, but still must be satisfied for accessing the property    
    
All markings from a conjunctive set must be met for access.
"""


ContainerDisjunctiveMarkingSummary: typing_extensions.TypeAlias = typing.List[
    typing.List["MarkingId"]
]
"""
The disjunctive set of markings for the container of this property value,
such as the project of a dataset. These markings may differ from the marking
on the actual property value, but still must be satisfied for accessing the property        
All markings from a conjunctive set must be met for access.

Disjunctive markings are represented as a conjunctive list of disjunctive sets.
The top-level set is a conjunction of sets, where each inner set should be 
treated as a unit where any marking within the set can satisfy the set.
All sets within the top level set should be satisfied.
"""


class ContainsAllTermsInOrderPrefixLastTerm(core.ModelBase):
    """
    Returns objects where the specified field contains all of the terms in the order provided,
    but they do have to be adjacent to each other.
    The last term can be a partial prefix match. Allows you to specify a property to query on
    by a variety of means. Either `field` or `propertyIdentifier` can be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: str
    type: typing.Literal["containsAllTermsInOrderPrefixLastTerm"] = (
        "containsAllTermsInOrderPrefixLastTerm"
    )


class ContainsAllTermsInOrderQuery(core.ModelBase):
    """
    Returns objects where the specified field contains all of the terms in the order provided,
    but they do have to be adjacent to each other. Allows you to specify a property to query on
    by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: str
    type: typing.Literal["containsAllTermsInOrder"] = "containsAllTermsInOrder"


class ContainsAllTermsQuery(core.ModelBase):
    """
    Returns objects where the specified field contains all of the whitespace separated words in any
    order in the provided value. This query supports fuzzy matching. Allows you to specify a property to query on
    by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: str
    fuzzy: typing.Optional[FuzzyV2] = None
    type: typing.Literal["containsAllTerms"] = "containsAllTerms"


class ContainsAnyTermQuery(core.ModelBase):
    """
    Returns objects where the specified field contains any of the whitespace separated words in any
    order in the provided value. This query supports fuzzy matching. Allows you to specify a property to query on
    by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: str
    fuzzy: typing.Optional[FuzzyV2] = None
    type: typing.Literal["containsAnyTerm"] = "containsAnyTerm"


class ContainsQueryV2(core.ModelBase):
    """
    Returns objects where the specified array contains a value. Allows you to specify a property to query on by a
    variety of means. Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: PropertyValue
    type: typing.Literal["contains"] = "contains"


class CountAggregationV2(core.ModelBase):
    """Computes the total count of objects."""

    name: typing.Optional[AggregationMetricName] = None
    direction: typing.Optional[OrderByDirection] = None
    type: typing.Literal["count"] = "count"


class CountObjectsResponseV2(core.ModelBase):
    """CountObjectsResponseV2"""

    count: typing.Optional[int] = None


class CreateEdit(core.ModelBase):
    """CreateEdit"""

    properties: typing.Dict[PropertyApiName, PropertyValue]
    """
    The property values of the object at the time of creation.
    Maps property API names to their values.
    """

    type: typing.Literal["createEdit"] = "createEdit"


class CreateInterfaceLinkLogicRule(core.ModelBase):
    """CreateInterfaceLinkLogicRule"""

    interface_type_api_name: InterfaceTypeApiName = pydantic.Field(alias=str("interfaceTypeApiName"))  # type: ignore[literal-required]
    interface_link_type_api_name: InterfaceLinkTypeApiName = pydantic.Field(alias=str("interfaceLinkTypeApiName"))  # type: ignore[literal-required]
    source_object: ParameterId = pydantic.Field(alias=str("sourceObject"))  # type: ignore[literal-required]
    target_object: ParameterId = pydantic.Field(alias=str("targetObject"))  # type: ignore[literal-required]
    type: typing.Literal["createInterfaceLink"] = "createInterfaceLink"


class CreateInterfaceLogicRule(core.ModelBase):
    """CreateInterfaceLogicRule"""

    interface_type_api_name: InterfaceTypeApiName = pydantic.Field(alias=str("interfaceTypeApiName"))  # type: ignore[literal-required]
    object_type: ParameterId = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]
    shared_property_arguments: typing.Dict[SharedPropertyTypeApiName, LogicRuleArgument] = pydantic.Field(alias=str("sharedPropertyArguments"))  # type: ignore[literal-required]
    struct_property_arguments: typing.Dict[SharedPropertyTypeApiName, typing.Dict[StructFieldApiName, StructFieldArgument]] = pydantic.Field(alias=str("structPropertyArguments"))  # type: ignore[literal-required]
    type: typing.Literal["createInterface"] = "createInterface"


class CreateInterfaceObjectRule(core.ModelBase):
    """CreateInterfaceObjectRule"""

    interface_type_api_name: InterfaceTypeApiName = pydantic.Field(alias=str("interfaceTypeApiName"))  # type: ignore[literal-required]
    type: typing.Literal["createInterfaceObject"] = "createInterfaceObject"


class CreateLinkLogicRule(core.ModelBase):
    """CreateLinkLogicRule"""

    link_type_api_name: LinkTypeApiName = pydantic.Field(alias=str("linkTypeApiName"))  # type: ignore[literal-required]
    source_object: ParameterId = pydantic.Field(alias=str("sourceObject"))  # type: ignore[literal-required]
    target_object: ParameterId = pydantic.Field(alias=str("targetObject"))  # type: ignore[literal-required]
    type: typing.Literal["createLink"] = "createLink"


class CreateLinkRule(core.ModelBase):
    """CreateLinkRule"""

    link_type_api_name_ato_b: LinkTypeApiName = pydantic.Field(alias=str("linkTypeApiNameAtoB"))  # type: ignore[literal-required]
    link_type_api_name_bto_a: LinkTypeApiName = pydantic.Field(alias=str("linkTypeApiNameBtoA"))  # type: ignore[literal-required]
    a_side_object_type_api_name: ObjectTypeApiName = pydantic.Field(alias=str("aSideObjectTypeApiName"))  # type: ignore[literal-required]
    b_side_object_type_api_name: ObjectTypeApiName = pydantic.Field(alias=str("bSideObjectTypeApiName"))  # type: ignore[literal-required]
    type: typing.Literal["createLink"] = "createLink"


class CreateObjectLogicRule(core.ModelBase):
    """CreateObjectLogicRule"""

    object_type_api_name: ObjectTypeApiName = pydantic.Field(alias=str("objectTypeApiName"))  # type: ignore[literal-required]
    property_arguments: typing.Dict[PropertyApiName, LogicRuleArgument] = pydantic.Field(alias=str("propertyArguments"))  # type: ignore[literal-required]
    struct_property_arguments: typing.Dict[PropertyApiName, typing.Dict[StructFieldApiName, StructFieldArgument]] = pydantic.Field(alias=str("structPropertyArguments"))  # type: ignore[literal-required]
    type: typing.Literal["createObject"] = "createObject"


class CreateObjectRule(core.ModelBase):
    """CreateObjectRule"""

    object_type_api_name: ObjectTypeApiName = pydantic.Field(alias=str("objectTypeApiName"))  # type: ignore[literal-required]
    type: typing.Literal["createObject"] = "createObject"


class CreateOntologyScenarioRequest(core.ModelBase):
    """The request payload for creating an ontology scenario."""

    base: typing.Optional[OntologyBase] = None
    expire_after: typing.Optional[core.AwareDatetime] = pydantic.Field(alias=str("expireAfter"), default=None)  # type: ignore[literal-required]
    """
    The date after which the scenario expires and is automatically cleaned up.
    Defaults to 31 days after creation. Must be after creation and no later
    than 31 days after creation.
    """


class CreateOntologyScenarioResponse(core.ModelBase):
    """The response payload for creating an ontology scenario."""

    scenario_rid: OntologyScenarioRid = pydantic.Field(alias=str("scenarioRid"))  # type: ignore[literal-required]


class CreateOrModifyObjectLogicRule(core.ModelBase):
    """CreateOrModifyObjectLogicRule"""

    object_type_api_name: ObjectTypeApiName = pydantic.Field(alias=str("objectTypeApiName"))  # type: ignore[literal-required]
    property_arguments: typing.Dict[PropertyApiName, LogicRuleArgument] = pydantic.Field(alias=str("propertyArguments"))  # type: ignore[literal-required]
    struct_property_arguments: typing.Dict[PropertyApiName, typing.Dict[StructFieldApiName, StructFieldArgument]] = pydantic.Field(alias=str("structPropertyArguments"))  # type: ignore[literal-required]
    type: typing.Literal["createOrModifyObject"] = "createOrModifyObject"


class CreateOrModifyObjectLogicRuleV2(core.ModelBase):
    """CreateOrModifyObjectLogicRuleV2"""

    object_to_modify: ParameterId = pydantic.Field(alias=str("objectToModify"))  # type: ignore[literal-required]
    property_arguments: typing.Dict[PropertyApiName, LogicRuleArgument] = pydantic.Field(alias=str("propertyArguments"))  # type: ignore[literal-required]
    struct_property_arguments: typing.Dict[PropertyApiName, typing.Dict[StructFieldApiName, StructFieldArgument]] = pydantic.Field(alias=str("structPropertyArguments"))  # type: ignore[literal-required]
    type: typing.Literal["createOrModifyObjectV2"] = "createOrModifyObjectV2"


class CreateTemporaryObjectSetRequestV2(core.ModelBase):
    """CreateTemporaryObjectSetRequestV2"""

    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]


class CreateTemporaryObjectSetResponseV2(core.ModelBase):
    """CreateTemporaryObjectSetResponseV2"""

    object_set_rid: ObjectSetRid = pydantic.Field(alias=str("objectSetRid"))  # type: ignore[literal-required]


class CurrentTimeArgument(core.ModelBase):
    """Represents the current time argument in a logic rule."""

    type: typing.Literal["currentTime"] = "currentTime"


class CurrentUserArgument(core.ModelBase):
    """Represents the current user argument in a logic rule."""

    type: typing.Literal["currentUser"] = "currentUser"


DataValue: typing_extensions.TypeAlias = typing.Any
"""
Represents the value of data in the following format. Note that these values can be nested, for example an array of structs.
| Type                                | JSON encoding                                         | Example                                                                                                                                                       |
|-------------------------------------|-------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Array                               | array                                                 | `["alpha", "bravo", "charlie"]`                                                                                                                               |
| Attachment                          | string                                                | `"ri.attachments.main.attachment.2f944bae-5851-4204-8615-920c969a9f2e"`                                                                                       |
| Boolean                             | boolean                                               | `true`                                                                                                                                                        |
| Byte                                | number                                                | `31`                                                                                                                                                          |
| CipherText                          | string                                                | `"CIPHER::ri.bellaso.main.cipher-channel.e414ab9e-b606-499a-a0e1-844fa296ba7e::unzjs3VifsTxuIpf1fH1CJ7OaPBr2bzMMdozPaZJtCii8vVG60yXIEmzoOJaEl9mfFFe::CIPHER"` |
| Date                                | ISO 8601 extended local date string                   | `"2021-05-01"`                                                                                                                                                |
| Decimal                             | string                                                | `"2.718281828"`                                                                                                                                               |
| Double                              | number                                                | `3.14159265`                                                                                                                                                  |
| EntrySet                            | array of JSON objects                                 | `[{"key": "EMP1234", "value": "true"}, {"key": "EMP4444", "value": "false"}]`                                                                                 |
| Float                               | number                                                | `3.14159265`                                                                                                                                                  |
| Integer                             | number                                                | `238940`                                                                                                                                                      |
| Long                                | string                                                | `"58319870951433"`                                                                                                                                            |
| Marking                             | string                                                | `"MU"`                                                                                                                                                        |
| Null                                | null                                                  | `null`                                                                                                                                                        |
| Object Set                          | string OR the object set definition                   | `ri.object-set.main.versioned-object-set.h13274m8-23f5-431c-8aee-a4554157c57z`                                                                                |
| Ontology Object Reference           | JSON encoding of the object's primary key             | `10033123` or `"EMP1234"`                                                                                                                                     |
| Ontology Interface Object Reference | JSON encoding of the object's API name and primary key| `{"objectTypeApiName":"Employee", "primaryKeyValue":"EMP1234"}`                                                                                               |
| Ontology Object Type Reference      | string of the object type's api name                  | `"Employee"`                                                                                                                                                  |
| Scenario Reference                  | string of the scenario RID                            | `"ri.actions..scenario.cf2a8a49-8b56-446d-ab04-a6bc7fadef48"`                                                                                                 |
| Set                                 | array                                                 | `["alpha", "bravo", "charlie"]`                                                                                                                               |
| Short                               | number                                                | `8739`                                                                                                                                                        |
| String                              | string                                                | `"Call me Ishmael"`                                                                                                                                           |
| Struct                              | JSON object                                           | `{"name": "John Doe", "age": 42}`                                                                                                                             |
| TwoDimensionalAggregation           | JSON object                                           | `{"groups": [{"key": "alpha", "value": 100}, {"key": "beta", "value": 101}]}`                                                                                 |
| ThreeDimensionalAggregation         | JSON object                                           | `{"groups": [{"key": "NYC", "groups": [{"key": "Engineer", "value" : 100}]}]}`                                                                                |
| Timestamp                           | ISO 8601 extended offset date-time string in UTC zone | `"2021-01-04T05:00:00Z"`                                                                                                                                      |
"""


DatasourceBranchId: typing_extensions.TypeAlias = str
"""The id of a datasource branch. Branch ids are user supplied strings, not RIDs."""


DatasourceRid: typing_extensions.TypeAlias = core.RID
"""Randomly generated identifier for an object type's datasource."""


class DateValue(core.ModelBase):
    """DateValue"""

    value: date
    type: typing.Literal["dateValue"] = "dateValue"


class DatetimeAllowedValues(core.ModelBase):
    """The parameter value must fall within the specified date or timestamp range."""

    gt: typing.Optional[ParameterDatetimeValue] = None
    gte: typing.Optional[ParameterDatetimeValue] = None
    lt: typing.Optional[ParameterDatetimeValue] = None
    lte: typing.Optional[ParameterDatetimeValue] = None
    type: typing.Literal["datetime"] = "datetime"


DatetimeFormat: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["DatetimeStringFormat", "DatetimeLocalizedFormat"],
    pydantic.Field(discriminator="type"),
]
"""DatetimeFormat"""


class DatetimeLocalizedFormat(core.ModelBase):
    """Predefined localized formatting options."""

    format: DatetimeLocalizedFormatType
    type: typing.Literal["localizedFormat"] = "localizedFormat"


DatetimeLocalizedFormatType: typing_extensions.TypeAlias = typing.Literal[
    "DATE_FORMAT_RELATIVE_TO_NOW",
    "DATE_FORMAT_DATE",
    "DATE_FORMAT_YEAR_AND_MONTH",
    "DATE_FORMAT_DATE_TIME",
    "DATE_FORMAT_DATE_TIME_SHORT",
    "DATE_FORMAT_TIME",
    "DATE_FORMAT_ISO_INSTANT",
]
"""Localized date/time format types."""


class DatetimeStringFormat(core.ModelBase):
    """A strictly specified date format pattern."""

    pattern: str
    """A valid format string composed of date/time patterns."""

    type: typing.Literal["stringFormat"] = "stringFormat"


DatetimeTimezone: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["DatetimeTimezoneStatic", "DatetimeTimezoneUser"],
    pydantic.Field(discriminator="type"),
]
"""DatetimeTimezone"""


class DatetimeTimezoneStatic(core.ModelBase):
    """DatetimeTimezoneStatic"""

    zone_id: PropertyTypeReferenceOrStringConstant = pydantic.Field(alias=str("zoneId"))  # type: ignore[literal-required]
    type: typing.Literal["static"] = "static"


class DatetimeTimezoneUser(core.ModelBase):
    """The user's local timezone."""

    type: typing.Literal["user"] = "user"


class DecryptionResult(core.ModelBase):
    """The result of a CipherText decryption. If successful, the plaintext decrypted value will be returned. Otherwise, an error will be thrown."""

    plaintext: Plaintext


class DeleteEdit(core.ModelBase):
    """DeleteEdit"""

    previous_properties: typing.Dict[PropertyApiName, PropertyValue] = pydantic.Field(alias=str("previousProperties"))  # type: ignore[literal-required]
    type: typing.Literal["deleteEdit"] = "deleteEdit"


class DeleteInterfaceLinkLogicRule(core.ModelBase):
    """DeleteInterfaceLinkLogicRule"""

    interface_type_api_name: InterfaceTypeApiName = pydantic.Field(alias=str("interfaceTypeApiName"))  # type: ignore[literal-required]
    interface_link_type_api_name: InterfaceLinkTypeApiName = pydantic.Field(alias=str("interfaceLinkTypeApiName"))  # type: ignore[literal-required]
    source_object: ParameterId = pydantic.Field(alias=str("sourceObject"))  # type: ignore[literal-required]
    target_object: ParameterId = pydantic.Field(alias=str("targetObject"))  # type: ignore[literal-required]
    type: typing.Literal["deleteInterfaceLink"] = "deleteInterfaceLink"


class DeleteInterfaceObjectRule(core.ModelBase):
    """DeleteInterfaceObjectRule"""

    interface_type_api_name: InterfaceTypeApiName = pydantic.Field(alias=str("interfaceTypeApiName"))  # type: ignore[literal-required]
    type: typing.Literal["deleteInterfaceObject"] = "deleteInterfaceObject"


class DeleteLink(core.ModelBase):
    """DeleteLink"""

    link_type_api_name_ato_b: LinkTypeApiName = pydantic.Field(alias=str("linkTypeApiNameAtoB"))  # type: ignore[literal-required]
    link_type_api_name_bto_a: LinkTypeApiName = pydantic.Field(alias=str("linkTypeApiNameBtoA"))  # type: ignore[literal-required]
    a_side_object: LinkSideObject = pydantic.Field(alias=str("aSideObject"))  # type: ignore[literal-required]
    b_side_object: LinkSideObject = pydantic.Field(alias=str("bSideObject"))  # type: ignore[literal-required]
    type: typing.Literal["deleteLink"] = "deleteLink"


class DeleteLinkEdit(core.ModelBase):
    """DeleteLinkEdit"""

    object_type: ObjectTypeApiName = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]
    primary_key: PrimaryKeyValue = pydantic.Field(alias=str("primaryKey"))  # type: ignore[literal-required]
    link_type: LinkTypeApiName = pydantic.Field(alias=str("linkType"))  # type: ignore[literal-required]
    linked_object_primary_key: PrimaryKeyValue = pydantic.Field(alias=str("linkedObjectPrimaryKey"))  # type: ignore[literal-required]
    type: typing.Literal["removeLink"] = "removeLink"


class DeleteLinkLogicRule(core.ModelBase):
    """DeleteLinkLogicRule"""

    link_type_api_name: LinkTypeApiName = pydantic.Field(alias=str("linkTypeApiName"))  # type: ignore[literal-required]
    source_object: ParameterId = pydantic.Field(alias=str("sourceObject"))  # type: ignore[literal-required]
    target_object: ParameterId = pydantic.Field(alias=str("targetObject"))  # type: ignore[literal-required]
    type: typing.Literal["deleteLink"] = "deleteLink"


class DeleteLinkRule(core.ModelBase):
    """DeleteLinkRule"""

    link_type_api_name_ato_b: LinkTypeApiName = pydantic.Field(alias=str("linkTypeApiNameAtoB"))  # type: ignore[literal-required]
    link_type_api_name_bto_a: LinkTypeApiName = pydantic.Field(alias=str("linkTypeApiNameBtoA"))  # type: ignore[literal-required]
    a_side_object_type_api_name: ObjectTypeApiName = pydantic.Field(alias=str("aSideObjectTypeApiName"))  # type: ignore[literal-required]
    b_side_object_type_api_name: ObjectTypeApiName = pydantic.Field(alias=str("bSideObjectTypeApiName"))  # type: ignore[literal-required]
    type: typing.Literal["deleteLink"] = "deleteLink"


class DeleteObject(core.ModelBase):
    """DeleteObject"""

    primary_key: PropertyValue = pydantic.Field(alias=str("primaryKey"))  # type: ignore[literal-required]
    object_type: ObjectTypeApiName = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]
    type: typing.Literal["deleteObject"] = "deleteObject"


class DeleteObjectEdit(core.ModelBase):
    """DeleteObjectEdit"""

    object_type: ObjectTypeApiName = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]
    primary_key: PropertyValue = pydantic.Field(alias=str("primaryKey"))  # type: ignore[literal-required]
    type: typing.Literal["deleteObject"] = "deleteObject"


class DeleteObjectLogicRule(core.ModelBase):
    """DeleteObjectLogicRule"""

    object_to_delete: ParameterId = pydantic.Field(alias=str("objectToDelete"))  # type: ignore[literal-required]
    type: typing.Literal["deleteObject"] = "deleteObject"


class DeleteObjectRule(core.ModelBase):
    """DeleteObjectRule"""

    object_type_api_name: ObjectTypeApiName = pydantic.Field(alias=str("objectTypeApiName"))  # type: ignore[literal-required]
    type: typing.Literal["deleteObject"] = "deleteObject"


class DeprecatedPropertyTypeStatus(core.ModelBase):
    """
    This status indicates that the PropertyType is reaching the end of its life and will be removed as per the
    deadline specified.
    """

    message: str
    deadline: core.AwareDatetime
    replaced_by: typing.Optional[PropertyTypeRid] = pydantic.Field(alias=str("replacedBy"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["deprecated"] = "deprecated"


DerivedPropertyApiName: typing_extensions.TypeAlias = str
"""The name of the derived property that will be returned."""


DerivedPropertyDefinition: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "AddPropertyExpression",
        "AbsoluteValuePropertyExpression",
        "ExtractPropertyExpression",
        "SelectedPropertyExpression",
        "NegatePropertyExpression",
        "SubtractPropertyExpression",
        "PropertyApiNameSelector",
        "LeastPropertyExpression",
        "DividePropertyExpression",
        "MultiplyPropertyExpression",
        "GreatestPropertyExpression",
    ],
    pydantic.Field(discriminator="type"),
]
"""Definition of a derived property."""


DirectSourceRid: typing_extensions.TypeAlias = core.RID
"""The RID of a direct-write source backing an object type."""


DisjunctiveMarkingSummary: typing_extensions.TypeAlias = typing.List[typing.List["MarkingId"]]
"""
The disjunctive set of markings required to access the property value.
Disjunctive markings are represented as a conjunctive list of disjunctive sets.
The top-level set is a conjunction of sets, where each inner set should be 
treated as a unit where any marking within the set can satisfy the set.
All sets within the top level set should be satisfied.
"""


class DividePropertyExpression(core.ModelBase):
    """Divides the left numeric value by the right numeric value."""

    left: DerivedPropertyDefinition
    right: DerivedPropertyDefinition
    type: typing.Literal["divide"] = "divide"


class DoesNotIntersectBoundingBoxQuery(core.ModelBase):
    """
    Returns objects where the specified field does not intersect the bounding box provided. Allows you to specify a
    property to query on by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not
    both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: BoundingBoxValue
    type: typing.Literal["doesNotIntersectBoundingBox"] = "doesNotIntersectBoundingBox"


class DoesNotIntersectPolygonQuery(core.ModelBase):
    """
    Returns objects where the specified field does not intersect the polygon provided. Allows you to specify a
    property to query on by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not
    both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: PolygonValue
    type: typing.Literal["doesNotIntersectPolygon"] = "doesNotIntersectPolygon"


class DoubleValue(core.ModelBase):
    """DoubleValue"""

    value: float
    type: typing.Literal["doubleValue"] = "doubleValue"


class DoubleVector(core.ModelBase):
    """
    The vector to search with. The vector must be of the same dimension as the vectors stored in the provided
    propertyIdentifier.
    """

    value: typing.List[float]
    type: typing.Literal["vector"] = "vector"


DurationBaseValue: typing_extensions.TypeAlias = typing.Literal["SECONDS", "MILLISECONDS"]
"""Specifies the unit of the input duration value."""


DurationFormatStyle: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["HumanReadableFormat", "TimeCodeFormat"], pydantic.Field(discriminator="type")
]
"""DurationFormatStyle"""


DurationPrecision: typing_extensions.TypeAlias = typing.Literal[
    "DAYS", "HOURS", "MINUTES", "SECONDS", "AUTO"
]
"""Specifies the maximum precision to apply when formatting a duration."""


EditHistoryEdit: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["CreateEdit", "DeleteEdit", "ModifyEdit"], pydantic.Field(discriminator="type")
]
"""EditHistoryEdit"""


class EditOnlyPropertyMapping(core.ModelBase):
    """A property on an object type that is permissioned to a tabular datasource, but the contents are only populated through Actions."""

    type: typing.Literal["editOnly"] = "editOnly"


EditsHistoryFilter: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["EditsHistoryTimestampFilter", "EditsHistoryOperationIdsFilter"],
    pydantic.Field(discriminator="type"),
]
"""EditsHistoryFilter"""


class EditsHistoryOperationIdsFilter(core.ModelBase):
    """EditsHistoryOperationIdsFilter"""

    operation_ids: typing.List[ActionRid] = pydantic.Field(alias=str("operationIds"))  # type: ignore[literal-required]
    type: typing.Literal["operationIdsFilter"] = "operationIdsFilter"


EditsHistorySortOrder: typing_extensions.TypeAlias = typing.Literal["newest_first", "oldest_first"]
"""EditsHistorySortOrder"""


class EditsHistoryTimestampFilter(core.ModelBase):
    """EditsHistoryTimestampFilter"""

    start_time: typing.Optional[core.AwareDatetime] = pydantic.Field(alias=str("startTime"), default=None)  # type: ignore[literal-required]
    """
    Filter edits to only those that occurred after this timestamp (inclusive).
    ISO 8601 format. Example: "2024-01-01T00:00:00Z"
    """

    end_time: typing.Optional[core.AwareDatetime] = pydantic.Field(alias=str("endTime"), default=None)  # type: ignore[literal-required]
    """
    Filter edits to only those that occurred before this timestamp (inclusive).
    ISO 8601 format. Example: "2024-12-31T23:59:59Z"
    """

    type: typing.Literal["timestampFilter"] = "timestampFilter"


class EncryptionRequest(core.ModelBase):
    """The request to encrypt a plaintext value into a CipherText value."""

    plaintext: Plaintext


class EncryptionResult(core.ModelBase):
    """The result of a CipherText encryption. If successful, the encrypted ciphertext value will be returned. Otherwise, an error will be thrown."""

    ciphertext: CipherText


class EntrySetType(core.ModelBase):
    """EntrySetType"""

    key_type: QueryDataType = pydantic.Field(alias=str("keyType"))  # type: ignore[literal-required]
    value_type: QueryDataType = pydantic.Field(alias=str("valueType"))  # type: ignore[literal-required]
    type: typing.Literal["entrySet"] = "entrySet"


class EnumConstraint(core.ModelBase):
    """EnumConstraint"""

    options: typing.List[typing.Optional[PropertyValue]]
    type: typing.Literal["enum"] = "enum"


class EqualsQueryV2(core.ModelBase):
    """
    Returns objects where the specified field is equal to a value. Allows you to specify a property to query on
    by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not both.

    For string properties, full term matching only works when **Selectable** is enabled for the property in Ontology Manager.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: PropertyValue
    type: typing.Literal["eq"] = "eq"


class Error(core.ModelBase):
    """Error"""

    error: ErrorName
    args: typing.List[Arg]
    type: typing.Literal["error"] = "error"


class ErrorComputingSecurity(core.ModelBase):
    """Indicates the server was not able to load the securities of the property."""

    type: typing.Literal["errorComputingSecurity"] = "errorComputingSecurity"


ErrorName: typing_extensions.TypeAlias = str
"""ErrorName"""


class ExactDistinctAggregationV2(core.ModelBase):
    """
    Computes an exact number of distinct values for the provided field. May be slower than an approximate
    distinct aggregation. Requires Object Storage V2.
    Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    name: typing.Optional[AggregationMetricName] = None
    direction: typing.Optional[OrderByDirection] = None
    type: typing.Literal["exactDistinct"] = "exactDistinct"


class ExamplePropertyTypeStatus(core.ModelBase):
    """
    This status indicates that the PropertyType is an example. It is backed by notional data that should not be
    used for actual workflows, but can be used to test those workflows.
    """

    type: typing.Literal["example"] = "example"


class ExecuteQueryRequest(core.ModelBase):
    """ExecuteQueryRequest"""

    parameters: typing.Dict[ParameterId, typing.Optional[DataValue]]


class ExecuteQueryResponse(core.ModelBase):
    """ExecuteQueryResponse"""

    value: DataValue


class ExperimentalPropertyTypeStatus(core.ModelBase):
    """This status indicates that the PropertyType is in development."""

    type: typing.Literal["experimental"] = "experimental"


ExtractDatePart: typing_extensions.TypeAlias = typing.Literal["DAYS", "MONTHS", "QUARTERS", "YEARS"]
"""ExtractDatePart"""


class ExtractMainValueLoadLevel(core.ModelBase):
    """Returns the main value of a struct as configured in the ontology."""

    type: typing.Literal["extractMainValue"] = "extractMainValue"


class ExtractPropertyExpression(core.ModelBase):
    """Extracts the specified date part from a date or timestamp."""

    property: DerivedPropertyDefinition
    part: ExtractDatePart
    type: typing.Literal["extract"] = "extract"


FilterValue: typing_extensions.TypeAlias = str
"""
Represents the value of a property filter. For instance, false is the FilterValue in
`properties.{propertyApiName}.isNull=false`.
"""


class FixedDatetimeValue(core.ModelBase):
    """An absolute datetime bound (ISO 8601 timestamp or date string)."""

    value: ParameterConstraintValue
    type: typing.Literal["fixed"] = "fixed"


FixedValuesMapKey: typing_extensions.TypeAlias = int
"""Integer key for fixed value mapping."""


class FullTextStringContainsPredicate(core.ModelBase):
    """
    Matches strings which contain the given string or parts of the given string. The exact behaviour can vary
    depending on the attribute searched for due to optimized text analysis.
    """

    value: str
    type: typing.Literal["contains"] = "contains"


class FullTextStringExactPredicate(core.ModelBase):
    """Matches strings representing the same sequence of characters as the given string."""

    value: str
    type: typing.Literal["exact"] = "exact"


FullTextStringPredicateV2: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["FullTextStringContainsPredicate", "FullTextStringExactPredicate"],
    pydantic.Field(discriminator="type"),
]
"""A predicate for matching strings."""


class FunctionLogicRule(core.ModelBase):
    """FunctionLogicRule"""

    function_rid: FunctionRid = pydantic.Field(alias=str("functionRid"))  # type: ignore[literal-required]
    function_version: FunctionVersion = pydantic.Field(alias=str("functionVersion"))  # type: ignore[literal-required]
    function_input_values: typing.Dict[FunctionParameterName, LogicRuleArgument] = pydantic.Field(alias=str("functionInputValues"))  # type: ignore[literal-required]
    type: typing.Literal["function"] = "function"


FunctionParameterName: typing_extensions.TypeAlias = str
"""The name of an input to a function."""


FunctionRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of a Function, useful for interacting with other Foundry APIs."""


class FunctionRidActionTypesQueryV2(core.ModelBase):
    """Returns action types which use the function with the given rid."""

    value: FunctionRid
    type: typing.Literal["functionRid"] = "functionRid"


FunctionVersion: typing_extensions.TypeAlias = str
"""
The version of the given Function, written `<major>.<minor>.<patch>-<tag>`, where `-<tag>` is optional.
Examples: `1.2.3`, `1.2.3-rc1`.
"""


class FuzzinessAuto(core.ModelBase):
    """
    Fuzzy search is activated, which can help discover additional results based on small differences in
    spelling, although some additional results may be less relevant.
    """

    type: typing.Literal["auto"] = "auto"


class FuzzinessOff(core.ModelBase):
    """
    Fuzzy search is turned off. Matches generated by modifying one or more characters of the search query are
    not returned.
    """

    type: typing.Literal["off"] = "off"


class FuzzyRule(core.ModelBase):
    """
    Matches intervals containing terms that are similar to the provided term, within an edit distance
    defined by fuzziness. An edit is a single character change needed to make a term match, including
    character insertion, deletion, substitution, or transposition of two adjacent characters.
    """

    term: str
    """The term to match."""

    fuzziness: typing.Optional[int] = None
    """Maximum edit distance allowed for matching. Valid values are 0, 1, or 2. Defaults to 2."""

    type: typing.Literal["fuzzy"] = "fuzzy"


FuzzyV2: typing_extensions.TypeAlias = bool
"""Setting fuzzy to `true` allows approximate matching in search queries that support it."""


class GeoJsonString(core.ModelBase):
    """A GeoJSON geometry specification."""

    geo_json: str = pydantic.Field(alias=str("geoJson"))  # type: ignore[literal-required]
    """
    A GeoJSON geometry string. Supported geometry types include Point, MultiPoint, LineString,
    MultiLineString, Polygon, MultiPolygon, and GeometryCollection.
    """

    type: typing.Literal["geoJson"] = "geoJson"


GeoShapeV2Geometry: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["BoundingBoxValue", "GeoJsonString"], pydantic.Field(discriminator="type")
]
"""Geometry specification for a GeoShapeV2Query. Supports bounding box envelopes and arbitrary GeoJSON geometries."""


class GeoShapeV2Query(core.ModelBase):
    """
    Returns objects where the specified field satisfies the provided geometry query with the given spatial operator.
    Supports both envelope (bounding box) and GeoJSON geometries for filtering geopoint or geoshape properties.
    Either `field` or `propertyIdentifier` can be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    geometry: GeoShapeV2Geometry
    spatial_filter_mode: SpatialFilterMode = pydantic.Field(alias=str("spatialFilterMode"))  # type: ignore[literal-required]
    type: typing.Literal["geoShapeV2"] = "geoShapeV2"


GeotemporalSeriesEntry: typing_extensions.TypeAlias = typing.Dict[
    "PropertyApiName", "PropertyValue"
]
"""
A single geotemporal data point. Each entry is a map from property API names to property values. Standard
entries include "time" (ISO 8601 timestamp) and "position" (GeoPoint), and may include additional geotemporal
series metadata fields such as speed, heading, or altitude.
"""


GeotimeSeriesIntegrationRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of a geotime integration."""


class GeotimeSeriesValue(core.ModelBase):
    """The underlying data values pointed to by a GeotimeSeriesReference."""

    position: geo_models.Position
    timestamp: core.AwareDatetime
    type: typing.Literal["geotimeSeriesValue"] = "geotimeSeriesValue"


class GetActionTypeByRidBatchRequest(core.ModelBase):
    """GetActionTypeByRidBatchRequest"""

    requests: typing_extensions.Annotated[
        typing.List[GetActionTypeByRidBatchRequestElement],
        annotated_types.Len(min_length=1, max_length=100),
    ]


class GetActionTypeByRidBatchRequestElement(core.ModelBase):
    """GetActionTypeByRidBatchRequestElement"""

    action_type_rid: ActionTypeRid = pydantic.Field(alias=str("actionTypeRid"))  # type: ignore[literal-required]


class GetActionTypeByRidBatchResponse(core.ModelBase):
    """GetActionTypeByRidBatchResponse"""

    data: typing.List[ActionTypeV2]


class GetActionTypeFullMetadataBatchRequest(core.ModelBase):
    """GetActionTypeFullMetadataBatchRequest"""

    requests: typing_extensions.Annotated[
        typing.List[GetActionTypeFullMetadataBatchRequestElement],
        annotated_types.Len(min_length=1, max_length=100),
    ]


class GetActionTypeFullMetadataBatchRequestElement(core.ModelBase):
    """GetActionTypeFullMetadataBatchRequestElement"""

    action_type: ActionTypeApiName = pydantic.Field(alias=str("actionType"))  # type: ignore[literal-required]


class GetActionTypeFullMetadataBatchResponse(core.ModelBase):
    """GetActionTypeFullMetadataBatchResponse"""

    data: typing.List[ActionTypeFullMetadata]


class GetObjectTypeByRidBatchRequest(core.ModelBase):
    """GetObjectTypeByRidBatchRequest"""

    requests: typing_extensions.Annotated[
        typing.List[GetObjectTypeByRidBatchRequestElement],
        annotated_types.Len(min_length=1, max_length=100),
    ]


class GetObjectTypeByRidBatchRequestElement(core.ModelBase):
    """GetObjectTypeByRidBatchRequestElement"""

    object_type_rid: ObjectTypeRid = pydantic.Field(alias=str("objectTypeRid"))  # type: ignore[literal-required]


class GetObjectTypeByRidBatchResponse(core.ModelBase):
    """GetObjectTypeByRidBatchResponse"""

    data: typing.List[ObjectTypeV2]


class GetObjectTypeFullMetadataBatchRequest(core.ModelBase):
    """GetObjectTypeFullMetadataBatchRequest"""

    requests: typing_extensions.Annotated[
        typing.List[GetObjectTypeFullMetadataBatchRequestElement],
        annotated_types.Len(min_length=1, max_length=100),
    ]
    include_link_types: typing.Optional[bool] = pydantic.Field(alias=str("includeLinkTypes"), default=None)  # type: ignore[literal-required]
    """
    A flag that will return all outgoing link types for each requested object type in that object type's
    `linkTypes`. When false or omitted, `linkTypes` is empty.
    """


class GetObjectTypeFullMetadataBatchRequestElement(core.ModelBase):
    """GetObjectTypeFullMetadataBatchRequestElement"""

    object_type: ObjectTypeApiName = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]


class GetObjectTypeFullMetadataBatchResponse(core.ModelBase):
    """GetObjectTypeFullMetadataBatchResponse"""

    data: typing.List[ObjectTypeFullMetadata]
    """
    The requested object types, in the order they were requested. Object types that were not found are
    omitted, so this may contain fewer entries than were requested.

    Each object type's `linkTypes` is only populated when the request specifies
    `includeLinkTypes=true`, and is ordered by link type API name. It may also be empty if the object
    type has no outgoing link types, or if the requesting token cannot see the object types on the
    other side of them.
    """


class GetOutgoingLinkTypesByObjectTypeRidBatchRequest(core.ModelBase):
    """GetOutgoingLinkTypesByObjectTypeRidBatchRequest"""

    requests: typing_extensions.Annotated[
        typing.List[GetOutgoingLinkTypesByObjectTypeRidBatchRequestElement],
        annotated_types.Len(min_length=1, max_length=100),
    ]
    filter_link_type_rids: typing.List[LinkTypeRid] = pydantic.Field(alias=str("filterLinkTypeRids"))  # type: ignore[literal-required]
    """
    If provided, only return outgoing link types with RIDs in this list.
    If omitted, all outgoing link types for each requested object type are returned.
    """


class GetOutgoingLinkTypesByObjectTypeRidBatchRequestElement(core.ModelBase):
    """GetOutgoingLinkTypesByObjectTypeRidBatchRequestElement"""

    object_type_rid: ObjectTypeRid = pydantic.Field(alias=str("objectTypeRid"))  # type: ignore[literal-required]


class GetOutgoingLinkTypesByObjectTypeRidBatchResponse(core.ModelBase):
    """GetOutgoingLinkTypesByObjectTypeRidBatchResponse"""

    data: typing.Dict[ObjectTypeRid, typing.List[LinkTypeSideV2]]


class GetQueryTypeByRidBatchRequest(core.ModelBase):
    """GetQueryTypeByRidBatchRequest"""

    requests: typing_extensions.Annotated[
        typing.List[GetQueryTypeByRidBatchRequestElement],
        annotated_types.Len(min_length=1, max_length=100),
    ]


class GetQueryTypeByRidBatchRequestElement(core.ModelBase):
    """GetQueryTypeByRidBatchRequestElement"""

    query_type_rid: FunctionRid = pydantic.Field(alias=str("queryTypeRid"))  # type: ignore[literal-required]
    function_version: typing.Optional[FunctionVersion] = pydantic.Field(alias=str("functionVersion"), default=None)  # type: ignore[literal-required]


class GetQueryTypeByRidBatchResponse(core.ModelBase):
    """GetQueryTypeByRidBatchResponse"""

    data: typing.List[QueryTypeV2]


class GetSelectedPropertyOperation(core.ModelBase):
    """
    Gets a single value of a property. Throws if the target object set is on the MANY side of the link and could
    explode the cardinality.

    Use collectList or collectSet which will return a list of values in that case.
    """

    selected_property_api_name: PropertyApiName = pydantic.Field(alias=str("selectedPropertyApiName"))  # type: ignore[literal-required]
    type: typing.Literal["get"] = "get"


class GreatestPropertyExpression(core.ModelBase):
    """Finds greatest of two or more numeric, date or timestamp values."""

    properties: typing.List[DerivedPropertyDefinition]
    type: typing.Literal["greatest"] = "greatest"


class GroupMemberConstraint(core.ModelBase):
    """The parameter value must be the user id of a member belonging to at least one of the groups defined by the constraint."""

    type: typing.Literal["groupMember"] = "groupMember"


class GtQueryV2(core.ModelBase):
    """
    Returns objects where the specified field is greater than a value. Allows you to specify a property to query on
    by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: PropertyValue
    type: typing.Literal["gt"] = "gt"


class GteQueryV2(core.ModelBase):
    """
    Returns objects where the specified field is greater than or equal to a value. Allows you to specify a property
    to query on by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: PropertyValue
    type: typing.Literal["gte"] = "gte"


class HasActionLogActionTypesQueryV2(core.ModelBase):
    """Returns action types based on whether they have an action log."""

    value: bool
    type: typing.Literal["hasActionLog"] = "hasActionLog"


class HasNotificationActionTypesQueryV2(core.ModelBase):
    """Returns action types based on whether they have a notification."""

    value: bool
    type: typing.Literal["hasNotification"] = "hasNotification"


class HasWebhookActionTypesQueryV2(core.ModelBase):
    """Returns action types based on whether they reference a webhook."""

    value: bool
    type: typing.Literal["hasWebhook"] = "hasWebhook"


class HumanReadableFormat(core.ModelBase):
    """Formats the duration as a human-readable written string."""

    show_full_units: typing.Optional[bool] = pydantic.Field(alias=str("showFullUnits"), default=None)  # type: ignore[literal-required]
    """Whether to show full or abbreviated time units."""

    type: typing.Literal["humanReadable"] = "humanReadable"


class InQuery(core.ModelBase):
    """
    Returns objects where the specified field equals any of the provided values. Allows you to
    specify a property to query on by a variety of means. If an empty array is provided as the value, then the filter will match all objects
    in the object set. Either `field` or `propertyIdentifier` must be supplied, but not both.

    For string properties, full term matching only works when **Selectable** is enabled for the property in Ontology Manager.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: typing.List[PropertyValue]
    type: typing.Literal["in"] = "in"


class InputObjectTypeRidActionTypesQueryV2(core.ModelBase):
    """Returns action types which reference the object type with the given rid as an input or product."""

    value: ObjectTypeRid
    type: typing.Literal["inputObjectTypeRid"] = "inputObjectTypeRid"


class IntegerValue(core.ModelBase):
    """IntegerValue"""

    value: int
    type: typing.Literal["integerValue"] = "integerValue"


InterfaceActionTypeConstraintApiName: typing_extensions.TypeAlias = str
"""
The name in the API of an action defined on an interface that implementing object types provide a concrete
action type for.
"""


class InterfaceDefinedPropertyType(core.ModelBase):
    """
    An interface property type with an additional field to indicate constraints that need to be satisfied by
    implementing object property types.
    """

    rid: InterfacePropertyTypeRid
    api_name: InterfacePropertyApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    display_name: core_models.DisplayName = pydantic.Field(alias=str("displayName"))  # type: ignore[literal-required]
    description: typing.Optional[str] = None
    """The description of the interface property type."""

    data_type: ObjectPropertyType = pydantic.Field(alias=str("dataType"))  # type: ignore[literal-required]
    value_type_api_name: typing.Optional[ValueTypeApiName] = pydantic.Field(alias=str("valueTypeApiName"), default=None)  # type: ignore[literal-required]
    require_implementation: bool = pydantic.Field(alias=str("requireImplementation"))  # type: ignore[literal-required]
    """Whether each implementing object type must declare an implementation for this property."""

    type_classes: typing.Optional[typing.List[TypeClass]] = pydantic.Field(alias=str("typeClasses"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["interfaceDefinedPropertyType"] = "interfaceDefinedPropertyType"


class InterfaceLinkType(core.ModelBase):
    """
    A link type constraint defined at the interface level where the implementation of the links is provided
    by the implementing object types.
    """

    rid: InterfaceLinkTypeRid
    api_name: InterfaceLinkTypeApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    display_name: core_models.DisplayName = pydantic.Field(alias=str("displayName"))  # type: ignore[literal-required]
    description: typing.Optional[str] = None
    """The description of the interface link type."""

    linked_entity_api_name: InterfaceLinkTypeLinkedEntityApiName = pydantic.Field(alias=str("linkedEntityApiName"))  # type: ignore[literal-required]
    cardinality: InterfaceLinkTypeCardinality
    required: bool
    """Whether each implementing object type must declare at least one implementation of this link."""


InterfaceLinkTypeApiName: typing_extensions.TypeAlias = str
"""
The name of the interface link type in the API. To find the API name for your Interface Link Type, check the 
[Ontology Manager](https://palantir.com/docs/foundry/ontology-manager/overview/).
"""


InterfaceLinkTypeCardinality: typing_extensions.TypeAlias = typing.Literal["ONE", "MANY"]
"""
The cardinality of the link in the given direction. Cardinality can be "ONE", meaning an object can
link to zero or one other objects, or "MANY", meaning an object can link to any number of other objects.
"""


InterfaceLinkTypeLinkedEntityApiName: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["LinkedObjectTypeApiName", "LinkedInterfaceTypeApiName"],
    pydantic.Field(discriminator="type"),
]
"""A reference to the linked entity. This can either be an object or an interface type."""


InterfaceLinkTypeRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of an interface link type, useful for interacting with other Foundry APIs."""


class InterfaceParameterPropertyArgument(core.ModelBase):
    """Represents an interface parameter property argument in a logic rule."""

    parameter_id: ParameterId = pydantic.Field(alias=str("parameterId"))  # type: ignore[literal-required]
    shared_property_type_rid: core.RID = pydantic.Field(alias=str("sharedPropertyTypeRid"))  # type: ignore[literal-required]
    type: typing.Literal["interfaceParameterPropertyValue"] = "interfaceParameterPropertyValue"


InterfacePropertyApiName: typing_extensions.TypeAlias = str
"""
The name of the interface property type in the API in lowerCamelCase format. To find the API name for your
interface property type, use the `List interface types` endpoint and check the `allPropertiesV2` field or check
the **Ontology Manager**.
"""


class InterfacePropertyLocalPropertyImplementation(core.ModelBase):
    """An implementation of an interface property via a local property."""

    property_api_name: PropertyApiName = pydantic.Field(alias=str("propertyApiName"))  # type: ignore[literal-required]
    type: typing.Literal["localPropertyImplementation"] = "localPropertyImplementation"


class InterfacePropertyReducedPropertyImplementation(core.ModelBase):
    """An implementation of an interface property via applying reducers on the nested implementation."""

    implementation: NestedInterfacePropertyTypeImplementation
    type: typing.Literal["reducedPropertyImplementation"] = "reducedPropertyImplementation"


class InterfacePropertyStructFieldImplementation(core.ModelBase):
    """An implementation of an interface property via the field of a local struct property."""

    struct_field_of_property: StructFieldOfPropertyImplementation = pydantic.Field(alias=str("structFieldOfProperty"))  # type: ignore[literal-required]
    type: typing.Literal["structFieldImplementation"] = "structFieldImplementation"


class InterfacePropertyStructImplementation(core.ModelBase):
    """
    An implementation of a struct interface property via a local struct property. Specifies a mapping of interface
    struct fields to local struct fields or properties.
    """

    mapping: InterfacePropertyStructImplementationMapping
    type: typing.Literal["structImplementation"] = "structImplementation"


InterfacePropertyStructImplementationMapping: typing_extensions.TypeAlias = typing.Dict[
    "StructFieldApiName", "PropertyOrStructFieldOfPropertyImplementation"
]
"""
An implementation of a struct interface property via a local struct property. Specifies a mapping of interface
struct fields to local struct fields or properties.
"""


InterfacePropertyType: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["InterfaceDefinedPropertyType", "InterfaceSharedPropertyType"],
    pydantic.Field(discriminator="type"),
]
"""
The definition of an interface property type on an interface. An interface property can either be backed by a
shared property type or defined on the interface directly.
"""


InterfacePropertyTypeImplementation: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "InterfacePropertyStructFieldImplementation",
        "InterfacePropertyStructImplementation",
        "InterfacePropertyLocalPropertyImplementation",
        "InterfacePropertyReducedPropertyImplementation",
    ],
    pydantic.Field(discriminator="type"),
]
"""Describes how an object type implements an interface property."""


InterfacePropertyTypeRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of an interface property type, useful for interacting with other Foundry APIs."""


class InterfaceSharedPropertyType(core.ModelBase):
    """
    A shared property type with an additional field to indicate whether the property must be included on every
    object type that implements the interface, or whether it is optional.
    """

    rid: SharedPropertyTypeRid
    api_name: SharedPropertyTypeApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    display_name: core_models.DisplayName = pydantic.Field(alias=str("displayName"))  # type: ignore[literal-required]
    description: typing.Optional[str] = None
    """A short text that describes the SharedPropertyType."""

    data_type: ObjectPropertyType = pydantic.Field(alias=str("dataType"))  # type: ignore[literal-required]
    value_type_api_name: typing.Optional[ValueTypeApiName] = pydantic.Field(alias=str("valueTypeApiName"), default=None)  # type: ignore[literal-required]
    value_formatting: typing.Optional[PropertyValueFormattingRule] = pydantic.Field(alias=str("valueFormatting"), default=None)  # type: ignore[literal-required]
    required: bool
    """Whether each implementing object type must declare an implementation for this property."""

    type_classes: typing.Optional[typing.List[TypeClass]] = pydantic.Field(alias=str("typeClasses"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["interfaceSharedPropertyType"] = "interfaceSharedPropertyType"


InterfaceToObjectTypeMapping: typing_extensions.TypeAlias = typing.Dict[
    "SharedPropertyTypeApiName", "PropertyApiName"
]
"""Represents an implementation of an interface (the mapping of interface property to local property)."""


InterfaceToObjectTypeMappingV2: typing_extensions.TypeAlias = typing.Dict[
    "InterfacePropertyApiName", "InterfacePropertyTypeImplementation"
]
"""Represents an implementation of an interface (the mapping of interface property to how it is implemented."""


InterfaceToObjectTypeMappings: typing_extensions.TypeAlias = typing.Dict[
    "ObjectTypeApiName", "InterfaceToObjectTypeMapping"
]
"""Map from object type to the interface-to-object-type mapping for that object type."""


InterfaceToObjectTypeMappingsV2: typing_extensions.TypeAlias = typing.Dict[
    "ObjectTypeApiName", "InterfaceToObjectTypeMappingV2"
]
"""Map from object type to the interface property implementations of that object type."""


class InterfaceType(core.ModelBase):
    """Represents an interface type in the Ontology."""

    rid: InterfaceTypeRid
    api_name: InterfaceTypeApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    display_name: core_models.DisplayName = pydantic.Field(alias=str("displayName"))  # type: ignore[literal-required]
    description: typing.Optional[str] = None
    """The description of the interface."""

    properties: typing.Dict[SharedPropertyTypeApiName, InterfaceSharedPropertyType]
    """
    A map from a shared property type API name to the corresponding shared property type. The map describes the 
    set of properties the interface has. A shared property type must be unique across all of the properties.
    This field only includes properties on the interface that are backed by shared property types.
    """

    all_properties: typing.Dict[SharedPropertyTypeApiName, InterfaceSharedPropertyType] = pydantic.Field(alias=str("allProperties"))  # type: ignore[literal-required]
    """
    A map from a shared property type API name to the corresponding shared property type. The map describes the 
    set of properties the interface has, including properties from all directly and indirectly extended 
    interfaces.
    This field only includes properties on the interface that are backed by shared property types.
    """

    properties_v2: typing.Dict[InterfacePropertyApiName, InterfacePropertyType] = pydantic.Field(alias=str("propertiesV2"))  # type: ignore[literal-required]
    """
    A map from a interface property type API name to the corresponding interface property type. The map
    describes the set of properties the interface has. An interface property can either be backed by a shared
    property or it can be defined directly on the interface.
    """

    all_properties_v2: typing.Dict[InterfacePropertyApiName, ResolvedInterfacePropertyType] = pydantic.Field(alias=str("allPropertiesV2"))  # type: ignore[literal-required]
    """
    A map from a interface property type API name to the corresponding interface property type. The map
    describes the set of properties the interface has, including properties from all directly and indirectly
    extended interfaces.
    """

    extends_interfaces: typing.List[InterfaceTypeApiName] = pydantic.Field(alias=str("extendsInterfaces"))  # type: ignore[literal-required]
    """
    A list of interface API names that this interface extends. An interface can extend other interfaces to 
    inherit their properties.
    """

    all_extends_interfaces: typing.List[InterfaceTypeApiName] = pydantic.Field(alias=str("allExtendsInterfaces"))  # type: ignore[literal-required]
    """A list of interface API names that this interface extends, both directly and indirectly."""

    implemented_by_object_types: typing.List[ObjectTypeApiName] = pydantic.Field(alias=str("implementedByObjectTypes"))  # type: ignore[literal-required]
    """A list of object API names that implement this interface."""

    links: typing.Dict[InterfaceLinkTypeApiName, InterfaceLinkType]
    """
    A map from an interface link type API name to the corresponding interface link type. The map describes the
    set of link types the interface has.
    """

    all_links: typing.Dict[InterfaceLinkTypeApiName, InterfaceLinkType] = pydantic.Field(alias=str("allLinks"))  # type: ignore[literal-required]
    """
    A map from an interface link type API name to the corresponding interface link type. The map describes the
    set of link types the interface has, including links from all directly and indirectly extended interfaces.
    """


InterfaceTypeApiName: typing_extensions.TypeAlias = str
"""
The name of the interface type in the API in UpperCamelCase format. To find the API name for your interface
type, use the `List interface types` endpoint or check the **Ontology Manager**.
"""


InterfaceTypeRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of an interface, useful for interacting with other Foundry APIs."""


class IntersectsBoundingBoxQuery(core.ModelBase):
    """
    Returns objects where the specified field intersects the bounding box provided. Allows you to specify a property
    to query on by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: BoundingBoxValue
    type: typing.Literal["intersectsBoundingBox"] = "intersectsBoundingBox"


class IntersectsPolygonQuery(core.ModelBase):
    """
    Returns objects where the specified field intersects the polygon provided. Allows you to specify a property to
    query on by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: PolygonValue
    type: typing.Literal["intersectsPolygon"] = "intersectsPolygon"


class IntervalQuery(core.ModelBase):
    """
    Returns objects where the specified field matches the sub-rule provided. This applies to the analyzed form of
    text fields. Either `field` or `propertyIdentifier` can be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    rule: IntervalQueryRule
    type: typing.Literal["interval"] = "interval"


IntervalQueryRule: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["AllOfRule", "MatchRule", "AnyOfRule", "PrefixOnLastTokenRule", "FuzzyRule"],
    pydantic.Field(discriminator="type"),
]
"""Sub-rule used for evaluating an IntervalQuery"""


class IsNullQueryV2(core.ModelBase):
    """
    Returns objects based on the existence of the specified field. Allows you to specify a property to query on
    by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: bool
    type: typing.Literal["isNull"] = "isNull"


KnownType: typing_extensions.TypeAlias = typing.Literal[
    "USER_OR_GROUP_ID", "RESOURCE_RID", "ARTIFACT_GID"
]
"""
Known Foundry types for specialized formatting:
- userOrGroupRid: Format as user or group
- resourceRid: Format as resource 
- artifactGid: Format as artifact
"""


class LeastPropertyExpression(core.ModelBase):
    """Finds least of two or more numeric, date or timestamp values."""

    properties: typing.List[DerivedPropertyDefinition]
    type: typing.Literal["least"] = "least"


class LengthConstraint(core.ModelBase):
    """LengthConstraint"""

    minimum_length: typing.Optional[float] = pydantic.Field(alias=str("minimumLength"), default=None)  # type: ignore[literal-required]
    maximum_length: typing.Optional[float] = pydantic.Field(alias=str("maximumLength"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["length"] = "length"


class LinkSideObject(core.ModelBase):
    """LinkSideObject"""

    primary_key: PropertyValue = pydantic.Field(alias=str("primaryKey"))  # type: ignore[literal-required]
    object_type: ObjectTypeApiName = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]


LinkTypeApiName: typing_extensions.TypeAlias = str
"""
The name of the link type in the API. To find the API name for your Link Type, check the **Ontology Manager**
application.
"""


LinkTypeId: typing_extensions.TypeAlias = str
"""The unique ID of a link type. To find the ID for your link type, check the **Ontology Manager** application."""


LinkTypeRid: typing_extensions.TypeAlias = core.RID
"""LinkTypeRid"""


LinkTypeSideCardinality: typing_extensions.TypeAlias = typing.Literal["ONE", "MANY"]
"""LinkTypeSideCardinality"""


class LinkTypeSideV2(core.ModelBase):
    """
    `foreignKeyPropertyApiName` is the API name of the foreign key on this object type. If absent, the link is
    either a m2m link or the linked object has the foreign key and this object type has the primary key.
    """

    api_name: LinkTypeApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    display_name: core_models.DisplayName = pydantic.Field(alias=str("displayName"))  # type: ignore[literal-required]
    status: core_models.ReleaseStatus
    object_type_api_name: ObjectTypeApiName = pydantic.Field(alias=str("objectTypeApiName"))  # type: ignore[literal-required]
    cardinality: LinkTypeSideCardinality
    foreign_key_property_api_name: typing.Optional[PropertyApiName] = pydantic.Field(alias=str("foreignKeyPropertyApiName"), default=None)  # type: ignore[literal-required]
    link_type_rid: LinkTypeRid = pydantic.Field(alias=str("linkTypeRid"))  # type: ignore[literal-required]


class LinkedInterfaceTypeApiName(core.ModelBase):
    """A reference to the linked interface type."""

    api_name: InterfaceTypeApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    type: typing.Literal["interfaceTypeApiName"] = "interfaceTypeApiName"


class LinkedObjectLocator(core.ModelBase):
    """
    Does not contain information about the source object. Should be used in a nested type that provides information about source objects.
    The `targetObject` Ontology Object in this response will only ever have the `__primaryKey` and `__apiName`
    fields present, thus functioning as object locators rather than full objects.
    """

    target_object: typing.Optional[OntologyObjectV2] = pydantic.Field(alias=str("targetObject"), default=None)  # type: ignore[literal-required]
    link_type: typing.Optional[LinkTypeApiName] = pydantic.Field(alias=str("linkType"), default=None)  # type: ignore[literal-required]


class LinkedObjectTypeApiName(core.ModelBase):
    """A reference to the linked object type."""

    api_name: ObjectTypeApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    type: typing.Literal["objectTypeApiName"] = "objectTypeApiName"


class LinksFromObject(core.ModelBase):
    """
    The Ontology Objects in this response will only ever have the `__primaryKey` and `__apiName`
    fields present, thus functioning as object locators rather than full objects.
    """

    source_object: typing.Optional[OntologyObjectV2] = pydantic.Field(alias=str("sourceObject"), default=None)  # type: ignore[literal-required]
    linked_objects: typing.List[LinkedObjectLocator] = pydantic.Field(alias=str("linkedObjects"))  # type: ignore[literal-required]


class ListActionTypesFullMetadataResponse(core.ModelBase):
    """ListActionTypesFullMetadataResponse"""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    data: typing.List[ActionTypeFullMetadata]


class ListActionTypesResponseV2(core.ModelBase):
    """ListActionTypesResponseV2"""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    data: typing.List[ActionTypeV2]


class ListAttachmentsResponseV2(core.ModelBase):
    """ListAttachmentsResponseV2"""

    data: typing.List[AttachmentV2]
    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["multiple"] = "multiple"


class ListInterfaceLinkedObjectsResponse(core.ModelBase):
    """ListInterfaceLinkedObjectsResponse"""

    data: typing.List[OntologyObjectV2]
    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]


class ListInterfaceTypesResponse(core.ModelBase):
    """ListInterfaceTypesResponse"""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    data: typing.List[InterfaceType]


class ListLinkedObjectsResponseV2(core.ModelBase):
    """ListLinkedObjectsResponseV2"""

    data: typing.List[OntologyObjectV2]
    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]


class ListObjectTypesV2Response(core.ModelBase):
    """ListObjectTypesV2Response"""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    data: typing.List[ObjectTypeV2]
    """The list of object types in the current page."""


class ListObjectsForInterfaceResponse(core.ModelBase):
    """ListObjectsForInterfaceResponse"""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    data: typing.List[OntologyObjectV2]
    """The list of interface instances in the current page."""

    total_count: core_models.TotalCount = pydantic.Field(alias=str("totalCount"))  # type: ignore[literal-required]


class ListObjectsResponseV2(core.ModelBase):
    """ListObjectsResponseV2"""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    data: typing.List[OntologyObjectV2]
    """The list of objects in the current page."""

    total_count: core_models.TotalCount = pydantic.Field(alias=str("totalCount"))  # type: ignore[literal-required]


class ListOntologiesV2Response(core.ModelBase):
    """ListOntologiesV2Response"""

    data: typing.List[OntologyV2]
    """The list of Ontologies the user has access to."""


class ListOntologyValueTypesResponse(core.ModelBase):
    """ListOntologyValueTypesResponse"""

    data: typing.List[OntologyValueType]


class ListOutgoingInterfaceLinkTypesResponse(core.ModelBase):
    """ListOutgoingInterfaceLinkTypesResponse"""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    data: typing.List[InterfaceLinkType]
    """The list of interface link types in the current page."""


class ListOutgoingLinkTypesResponseV2(core.ModelBase):
    """ListOutgoingLinkTypesResponseV2"""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    data: typing.List[LinkTypeSideV2]
    """The list of link type sides in the current page."""


class ListQueryTypesResponseV2(core.ModelBase):
    """ListQueryTypesResponseV2"""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    data: typing.List[QueryTypeV2]


class ListScenarioConflictingObjectsResponse(core.ModelBase):
    """The objects that have a conflicting edit within a scenario for a given object type."""

    data: typing.List[ObjectLocator]
    """The list of objects that have a conflicting edit within the scenario."""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]


class ListScenarioEditedEntityTypesResponse(core.ModelBase):
    """The object types and link types that have been modified within a scenario."""

    object_types: typing.List[ObjectTypeApiName] = pydantic.Field(alias=str("objectTypes"))  # type: ignore[literal-required]
    """The list of object type API names that have been modified within the scenario."""

    link_types: typing.List[ObjectTypeLinkTypeApiNameMapping] = pydantic.Field(alias=str("linkTypes"))  # type: ignore[literal-required]
    """
    The list of edited link types grouped by source object type.

    Note that only many-to-many link types are returned. One-to-many link type edits are surfaced as object
    edits on the object type that owns the foreign key property.
    """


class ListScenarioEditedLinkTypesResponse(core.ModelBase):
    """The link types that have been modified within a scenario."""

    data: typing.List[LinkTypeApiName]
    """The list of link type API names that have been modified within the scenario."""


class ListScenarioEditedLinksResponse(core.ModelBase):
    """The linked objects that have been edited within a scenario for a given link type, grouped by source object."""

    data: typing.List[LinksFromObject]
    """The list of linked objects that have been edited within the scenario, grouped by source object."""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]


class ListScenarioEditedObjectTypesResponse(core.ModelBase):
    """The object types that have been modified within a scenario."""

    data: typing.List[ObjectTypeApiName]
    """The list of object type API names that have been modified within the scenario."""


class ListScenarioEditedObjectsResponse(core.ModelBase):
    """
    The objects that have been edited within a scenario for a given object type.

    The Ontology Objects in this response will only ever have the `__primaryKey` and `__apiName`
    fields present, thus functioning as object locators rather than full objects.
    """

    data: typing.List[OntologyObjectV2]
    """The list of objects that have been edited within the scenario."""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]


class LoadGeotemporalSeriesRequest(core.ModelBase):
    """
    The request body for loading entries from a geotemporal series reference property.

    A geotemporal series represents time-indexed geographic observations for an object, such as the location history
    of a vehicle or aircraft. Each entry in the response is a map of property names to values, following the same
    structure as `OntologyObjectV2`.

    The `range` field is required and restricts results to a specific time window. Both `startTime` and `endTime`
    are required on `range`. The `additionalProperties` field controls which additional properties appear in each
    returned entry. Results are paginated; use `pageToken` from a previous response to retrieve additional pages.
    """

    range: AbsoluteTimeRange
    additional_properties: typing.List[SelectedPropertyApiName] = pydantic.Field(alias=str("additionalProperties"))  # type: ignore[literal-required]
    """
    The additional property API names to include in each entry. The "time" and "position" properties are always
    included and do not need to be specified here. Use this to request additional geotemporal series metadata
    properties such as "speed" or "heading". Properties that are not available for the underlying geotemporal
    integration will be omitted from the response entries.
    """

    page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("pageToken"), default=None)  # type: ignore[literal-required]
    page_size: typing.Optional[core_models.PageSize] = pydantic.Field(alias=str("pageSize"), default=None)  # type: ignore[literal-required]


class LoadGeotemporalSeriesResponse(core.ModelBase):
    """
    The response when loading entries from a geotemporal series reference property.

    Each entry in `data` is a map of property names to values containing the fields requested via
    `additionalProperties` in the corresponding `LoadGeotemporalSeriesRequest`. If `nextPageToken` is present,
    additional entries are available and can be retrieved by passing the token in a subsequent request.
    """

    data: typing.List[GeotemporalSeriesEntry]
    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]


class LoadObjectSetLinksRequestV2(core.ModelBase):
    """LoadObjectSetLinksRequestV2"""

    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    links: typing.List[LinkTypeApiName]
    page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("pageToken"), default=None)  # type: ignore[literal-required]
    include_compute_usage: typing.Optional[core_models.IncludeComputeUsage] = pydantic.Field(alias=str("includeComputeUsage"), default=None)  # type: ignore[literal-required]


class LoadObjectSetLinksResponseV2(core.ModelBase):
    """LoadObjectSetLinksResponseV2"""

    data: typing.List[LinksFromObject]
    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    compute_usage: typing.Optional[core_models.ComputeSeconds] = pydantic.Field(alias=str("computeUsage"), default=None)  # type: ignore[literal-required]


class LoadObjectSetRequestV2(core.ModelBase):
    """Represents the API POST body when loading an `ObjectSet`."""

    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    order_by: typing.Optional[SearchOrderByV2] = pydantic.Field(alias=str("orderBy"), default=None)  # type: ignore[literal-required]
    select: typing.List[SelectedPropertyApiName]
    select_v2: typing.Optional[typing.List[PropertyIdentifier]] = pydantic.Field(alias=str("selectV2"), default=None)  # type: ignore[literal-required]
    """
    The identifiers of the properties to include in the response. Only selectV2 or select should be populated,
    but not both.
    """

    default_load_level: typing.Optional[PropertyLoadLevel] = pydantic.Field(alias=str("defaultLoadLevel"), default=None)  # type: ignore[literal-required]
    load_ontology_defined_derived_properties: typing.Optional[bool] = pydantic.Field(alias=str("loadOntologyDefinedDerivedProperties"), default=None)  # type: ignore[literal-required]
    """
    A flag to load ontology-defined derived properties (OTDPs) in the response. Defaults to true.
    Only applies when no explicit property selection (`select`/`selectV2`) is provided; when specific
    properties are selected, this flag has no effect and the selected properties are always returned.

    This feature is experimental and not yet generally available.
    """

    page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("pageToken"), default=None)  # type: ignore[literal-required]
    page_size: typing.Optional[core_models.PageSize] = pydantic.Field(alias=str("pageSize"), default=None)  # type: ignore[literal-required]
    exclude_rid: typing.Optional[bool] = pydantic.Field(alias=str("excludeRid"), default=None)  # type: ignore[literal-required]
    """
    A flag to exclude the retrieval of the `__rid` property.
    Setting this to true may improve performance of this endpoint for object types in OSV2.
    """

    load_property_securities: typing.Optional[bool] = pydantic.Field(alias=str("loadPropertySecurities"), default=None)  # type: ignore[literal-required]
    """
    A flag to load the securities for all properties.
    Setting this flag to true will return a list of securities in the `propertySecurities` field of the response.
    Returned objects will return all properties as Secured Property Values, which provide the property data
    as well an index into the `propertySecurities` list.
    This feature is experimental and not yet generally available.
    """

    snapshot: typing.Optional[bool] = None
    """
    A flag to use snapshot consistency when paging.
    Setting this to true will give you a consistent view from before you start paging through the results, ensuring you do not get duplicate or missing items.
    Setting this to false will let new results enter as you page, but you may encounter duplicate or missing items.
    This defaults to false if not specified, which means you will always get the latest results.
    """

    include_compute_usage: typing.Optional[core_models.IncludeComputeUsage] = pydantic.Field(alias=str("includeComputeUsage"), default=None)  # type: ignore[literal-required]
    reference_signing_options: typing.Optional[ReferenceSigningOptions] = pydantic.Field(alias=str("referenceSigningOptions"), default=None)  # type: ignore[literal-required]


class LoadObjectSetResponseV2(core.ModelBase):
    """Represents the API response when loading an `ObjectSet`."""

    data: typing.List[OntologyObjectV2]
    """The list of objects in the current Page."""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    total_count: core_models.TotalCount = pydantic.Field(alias=str("totalCount"))  # type: ignore[literal-required]
    compute_usage: typing.Optional[core_models.ComputeSeconds] = pydantic.Field(alias=str("computeUsage"), default=None)  # type: ignore[literal-required]
    property_securities: typing.Optional[typing.List[PropertySecurities]] = pydantic.Field(alias=str("propertySecurities"), default=None)  # type: ignore[literal-required]


class LoadObjectSetV2MultipleObjectTypesRequest(core.ModelBase):
    """Represents the API POST body when loading an `ObjectSet`. Used on the `/loadObjectsMultipleObjectTypes` endpoint only."""

    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    order_by: typing.Optional[SearchOrderByV2] = pydantic.Field(alias=str("orderBy"), default=None)  # type: ignore[literal-required]
    select: typing.List[SelectedPropertyApiName]
    select_v2: typing.Optional[typing.List[PropertyIdentifier]] = pydantic.Field(alias=str("selectV2"), default=None)  # type: ignore[literal-required]
    """
    The identifiers of the properties to include in the response. Only selectV2 or select should be populated,
    but not both.
    """

    default_load_level: typing.Optional[PropertyLoadLevel] = pydantic.Field(alias=str("defaultLoadLevel"), default=None)  # type: ignore[literal-required]
    load_ontology_defined_derived_properties: typing.Optional[bool] = pydantic.Field(alias=str("loadOntologyDefinedDerivedProperties"), default=None)  # type: ignore[literal-required]
    """
    A flag to load ontology-defined derived properties (OTDPs) in the response. Defaults to true.
    Only applies when no explicit property selection (`select`/`selectV2`) is provided; when specific
    properties are selected, this flag has no effect and the selected properties are always returned.

    This flag does not affect interface properties that are implemented by an OTDP on an
    object type; those are always returned regardless of this flag.

    This feature is experimental and not yet generally available.
    """

    page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("pageToken"), default=None)  # type: ignore[literal-required]
    page_size: typing.Optional[core_models.PageSize] = pydantic.Field(alias=str("pageSize"), default=None)  # type: ignore[literal-required]
    exclude_rid: typing.Optional[bool] = pydantic.Field(alias=str("excludeRid"), default=None)  # type: ignore[literal-required]
    """
    A flag to exclude the retrieval of the `$rid` property.
    Setting this to true may improve performance of this endpoint for object types in OSV2.
    """

    load_property_securities: typing.Optional[bool] = pydantic.Field(alias=str("loadPropertySecurities"), default=None)  # type: ignore[literal-required]
    """
    A flag to load the securities for all properties.
    Setting this flag to true will return a list of securities in the `propertySecurities` field of the response.
    Returned objects will return all properties as Secured Property Values, which provide the property data
    as well an index into the `propertySecurities` list.
    This feature is experimental and not yet generally available.
    """

    snapshot: typing.Optional[bool] = None
    """
    A flag to use snapshot consistency when paging.
    Setting this to true will give you a consistent view from before you start paging through the results, ensuring you do not get duplicate or missing items.
    Setting this to false will let new results enter as you page, but you may encounter duplicate or missing items.
    This defaults to false if not specified, which means you will always get the latest results.
    """

    include_compute_usage: typing.Optional[core_models.IncludeComputeUsage] = pydantic.Field(alias=str("includeComputeUsage"), default=None)  # type: ignore[literal-required]
    reference_signing_options: typing.Optional[ReferenceSigningOptions] = pydantic.Field(alias=str("referenceSigningOptions"), default=None)  # type: ignore[literal-required]


class LoadObjectSetV2MultipleObjectTypesResponse(core.ModelBase):
    """
    Represents the API response when loading an `ObjectSet`. An `interfaceToObjectTypeMappings` field is
    optionally returned if the type scope of the returned object set includes any interfaces. The "type scope"
    of an object set refers to whether objects contain all their properties (object-type type scope) or just the
    properties that implement interface properties (interface type scope). There can be multiple type scopes in a
    single object set- some objects may have all their properties and some may only have interface properties.

    The `interfaceToObjectTypeMappings` field contains mappings from `SharedPropertyTypeApiName`s on the interface(s) to
    `PropertyApiName` for properties on the object(s).

    The `interfaceToObjectTypeMappingsV2` field contains mappings from `InterfacePropertyApiName`s on the
    interface(s) to `InterfacePropertyTypeImplementation` for properties on the object(s). This therefore includes
    implementations of both properties backed by SharedPropertyTypes as well as properties defined on the interface.
    """

    data: typing.List[OntologyObjectV2]
    """The list of objects in the current page."""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    total_count: core_models.TotalCount = pydantic.Field(alias=str("totalCount"))  # type: ignore[literal-required]
    interface_to_object_type_mappings: typing.Dict[InterfaceTypeApiName, InterfaceToObjectTypeMappings] = pydantic.Field(alias=str("interfaceToObjectTypeMappings"))  # type: ignore[literal-required]
    interface_to_object_type_mappings_v2: typing.Dict[InterfaceTypeApiName, InterfaceToObjectTypeMappingsV2] = pydantic.Field(alias=str("interfaceToObjectTypeMappingsV2"))  # type: ignore[literal-required]
    compute_usage: typing.Optional[core_models.ComputeSeconds] = pydantic.Field(alias=str("computeUsage"), default=None)  # type: ignore[literal-required]
    property_securities: typing.Optional[typing.List[PropertySecurities]] = pydantic.Field(alias=str("propertySecurities"), default=None)  # type: ignore[literal-required]


class LoadObjectSetV2ObjectsOrInterfacesRequest(core.ModelBase):
    """Represents the API POST body when loading an `ObjectSet`. Used on the `/loadObjectsOrInterfaces` endpoint only."""

    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    order_by: typing.Optional[SearchOrderByV2] = pydantic.Field(alias=str("orderBy"), default=None)  # type: ignore[literal-required]
    select: typing.List[SelectedPropertyApiName]
    select_v2: typing.Optional[typing.List[PropertyIdentifier]] = pydantic.Field(alias=str("selectV2"), default=None)  # type: ignore[literal-required]
    """
    The identifiers of the properties to include in the response. Only selectV2 or select should be populated,
    but not both.
    """

    default_load_level: typing.Optional[PropertyLoadLevel] = pydantic.Field(alias=str("defaultLoadLevel"), default=None)  # type: ignore[literal-required]
    load_ontology_defined_derived_properties: typing.Optional[bool] = pydantic.Field(alias=str("loadOntologyDefinedDerivedProperties"), default=None)  # type: ignore[literal-required]
    """
    A flag to load ontology-defined derived properties (OTDPs) in the response. Defaults to true.
    Only applies when no explicit property selection (`select`/`selectV2`) is provided; when specific
    properties are selected, this flag has no effect and the selected properties are always returned.

    This flag does not affect interface properties that are implemented by an OTDP on an
    object type; those are always returned regardless of this flag.

    This feature is experimental and not yet generally available.
    """

    page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("pageToken"), default=None)  # type: ignore[literal-required]
    page_size: typing.Optional[core_models.PageSize] = pydantic.Field(alias=str("pageSize"), default=None)  # type: ignore[literal-required]
    exclude_rid: typing.Optional[bool] = pydantic.Field(alias=str("excludeRid"), default=None)  # type: ignore[literal-required]
    """
    A flag to exclude the retrieval of the `$rid` property.
    Setting this to true may improve performance of this endpoint for object types in OSV2.
    """

    snapshot: typing.Optional[bool] = None
    """
    A flag to use snapshot consistency when paging.
    Setting this to true will give you a consistent view from before you start paging through the results, ensuring you do not get duplicate or missing items.
    Setting this to false will let new results enter as you page, but you may encounter duplicate or missing items.
    This defaults to false if not specified, which means you will always get the latest results.
    """

    reference_signing_options: typing.Optional[ReferenceSigningOptions] = pydantic.Field(alias=str("referenceSigningOptions"), default=None)  # type: ignore[literal-required]


class LoadObjectSetV2ObjectsOrInterfacesResponse(core.ModelBase):
    """
    Represents the API response when loading an `ObjectSet`. Objects in the returned set can either have properties
    defined by an interface that the objects belong to or properties defined by the object type of the object.
    """

    data: typing.List[OntologyObjectV2]
    """The list of objects in the current page."""

    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    total_count: core_models.TotalCount = pydantic.Field(alias=str("totalCount"))  # type: ignore[literal-required]
    transaction_id: typing.Optional[OntologyTransactionId] = pydantic.Field(alias=str("transactionId"), default=None)  # type: ignore[literal-required]


class LoadOntologyMetadataRequest(core.ModelBase):
    """The Ontology metadata (i.e., object, link, action, query, and interface types) to load."""

    object_types: typing.List[ObjectTypeApiName] = pydantic.Field(alias=str("objectTypes"))  # type: ignore[literal-required]
    link_types: typing.List[LinkTypeApiName] = pydantic.Field(alias=str("linkTypes"))  # type: ignore[literal-required]
    action_types: typing.List[ActionTypeApiName] = pydantic.Field(alias=str("actionTypes"))  # type: ignore[literal-required]
    query_types: typing.List[VersionedQueryTypeApiName] = pydantic.Field(alias=str("queryTypes"))  # type: ignore[literal-required]
    interface_types: typing.List[InterfaceTypeApiName] = pydantic.Field(alias=str("interfaceTypes"))  # type: ignore[literal-required]
    include_action_type_full_metadata: typing.Optional[bool] = pydantic.Field(alias=str("includeActionTypeFullMetadata"), default=None)  # type: ignore[literal-required]
    """
    When set to `true`, the `actionTypesFullMetadata` field of the response will be populated in addition to
    `actionTypes`.
    """


LogicRule: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "DeleteInterfaceObjectRule",
        "ModifyInterfaceObjectRule",
        "ModifyObjectRule",
        "DeleteObjectRule",
        "CreateInterfaceObjectRule",
        "DeleteLinkRule",
        "CreateObjectRule",
        "CreateLinkRule",
        "ApplyScenarioRule",
    ],
    pydantic.Field(discriminator="type"),
]
"""LogicRule"""


class LogicRuleActionTypesQueryV2(core.ModelBase):
    """Returns action types which contain a logic rule of the given type."""

    value: ActionTypeLogicRuleTypeFilter
    type: typing.Literal["logicRule"] = "logicRule"


LogicRuleArgument: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "CurrentTimeArgument",
        "StaticArgument",
        "CurrentUserArgument",
        "ParameterIdArgument",
        "InterfaceParameterPropertyArgument",
        "SynchronousWebhookOutputArgument",
        "ObjectParameterPropertyArgument",
        "UniqueIdentifierArgument",
    ],
    pydantic.Field(discriminator="type"),
]
"""Represents an argument for a logic rule operation. An argument can be passed in via the action parameters, as a static value, or as some other value."""


class LongValue(core.ModelBase):
    """LongValue"""

    value: core.Long
    type: typing.Literal["longValue"] = "longValue"


class LtQueryV2(core.ModelBase):
    """
    Returns objects where the specified field is less than a value. Allows you to specify a property to query on
    by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: PropertyValue
    type: typing.Literal["lt"] = "lt"


class LteQueryV2(core.ModelBase):
    """
    Returns objects where the specified field is less than or equal to a value. Allows you to specify a property to
    query on by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: PropertyValue
    type: typing.Literal["lte"] = "lte"


class MarkdownAllowedValues(core.ModelBase):
    """The parameter value (a markdown-formatted string) must satisfy the configured length bounds."""

    gte: typing.Optional[int] = None
    """Character length greater than or equal."""

    lte: typing.Optional[int] = None
    """Character length less than or equal."""

    type: typing.Literal["markdown"] = "markdown"


MarkingId: typing_extensions.TypeAlias = str
"""The id of a classification or mandatory marking."""


class MatchRule(core.ModelBase):
    """Matches intervals containing the terms in the query"""

    query: str
    max_gaps: typing.Optional[int] = pydantic.Field(alias=str("maxGaps"), default=None)  # type: ignore[literal-required]
    """
    The maximum gaps between matched terms in the interval. For example, in the text "quick brown fox",
    the terms "quick" and "fox" have a gap of one. If not set, then gaps are not considered.
    """

    ordered: bool
    """If true, the matched terms must occur in order."""

    type: typing.Literal["match"] = "match"


class MaxAggregationV2(core.ModelBase):
    """
    Computes the maximum value for the provided field.
    Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    name: typing.Optional[AggregationMetricName] = None
    direction: typing.Optional[OrderByDirection] = None
    type: typing.Literal["max"] = "max"


class MediaMetadata(core.ModelBase):
    """MediaMetadata"""

    path: typing.Optional[core_models.MediaItemPath] = None
    size_bytes: core_models.SizeBytes = pydantic.Field(alias=str("sizeBytes"))  # type: ignore[literal-required]
    media_type: core_models.MediaType = pydantic.Field(alias=str("mediaType"))  # type: ignore[literal-required]


class MinAggregationV2(core.ModelBase):
    """
    Computes the minimum value for the provided field.
    Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    name: typing.Optional[AggregationMetricName] = None
    direction: typing.Optional[OrderByDirection] = None
    type: typing.Literal["min"] = "min"


class ModifyEdit(core.ModelBase):
    """ModifyEdit"""

    includes_all_previous_values: typing.Optional[bool] = pydantic.Field(alias=str("includesAllPreviousValues"), default=None)  # type: ignore[literal-required]
    previous_properties: typing.Dict[PropertyApiName, PropertyValue] = pydantic.Field(alias=str("previousProperties"))  # type: ignore[literal-required]
    """
    The property values before the modification.
    Only includes properties that were changed.
    Maps property API names to their previous values.
    """

    properties: typing.Dict[PropertyApiName, PropertyValue]
    type: typing.Literal["modifyEdit"] = "modifyEdit"


class ModifyInterfaceLogicRule(core.ModelBase):
    """ModifyInterfaceLogicRule"""

    interface_object_to_modify: ParameterId = pydantic.Field(alias=str("interfaceObjectToModify"))  # type: ignore[literal-required]
    shared_property_arguments: typing.Dict[SharedPropertyTypeApiName, LogicRuleArgument] = pydantic.Field(alias=str("sharedPropertyArguments"))  # type: ignore[literal-required]
    struct_property_arguments: typing.Dict[SharedPropertyTypeApiName, typing.Dict[StructFieldApiName, StructFieldArgument]] = pydantic.Field(alias=str("structPropertyArguments"))  # type: ignore[literal-required]
    type: typing.Literal["modifyInterface"] = "modifyInterface"


class ModifyInterfaceObjectRule(core.ModelBase):
    """ModifyInterfaceObjectRule"""

    interface_type_api_name: InterfaceTypeApiName = pydantic.Field(alias=str("interfaceTypeApiName"))  # type: ignore[literal-required]
    type: typing.Literal["modifyInterfaceObject"] = "modifyInterfaceObject"


class ModifyObject(core.ModelBase):
    """ModifyObject"""

    primary_key: PropertyValue = pydantic.Field(alias=str("primaryKey"))  # type: ignore[literal-required]
    object_type: ObjectTypeApiName = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]
    type: typing.Literal["modifyObject"] = "modifyObject"


class ModifyObjectEdit(core.ModelBase):
    """ModifyObjectEdit"""

    object_type: ObjectTypeApiName = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]
    primary_key: PropertyValue = pydantic.Field(alias=str("primaryKey"))  # type: ignore[literal-required]
    properties: typing.Dict[PropertyApiName, typing.Optional[DataValue]]
    type: typing.Literal["modifyObject"] = "modifyObject"


class ModifyObjectLogicRule(core.ModelBase):
    """ModifyObjectLogicRule"""

    object_to_modify: ParameterId = pydantic.Field(alias=str("objectToModify"))  # type: ignore[literal-required]
    property_arguments: typing.Dict[PropertyApiName, LogicRuleArgument] = pydantic.Field(alias=str("propertyArguments"))  # type: ignore[literal-required]
    struct_property_arguments: typing.Dict[PropertyApiName, typing.Dict[StructFieldApiName, StructFieldArgument]] = pydantic.Field(alias=str("structPropertyArguments"))  # type: ignore[literal-required]
    type: typing.Literal["modifyObject"] = "modifyObject"


class ModifyObjectRule(core.ModelBase):
    """ModifyObjectRule"""

    object_type_api_name: ObjectTypeApiName = pydantic.Field(alias=str("objectTypeApiName"))  # type: ignore[literal-required]
    type: typing.Literal["modifyObject"] = "modifyObject"


class MultiplyPropertyExpression(core.ModelBase):
    """Multiplies two or more numeric values."""

    properties: typing.List[DerivedPropertyDefinition]
    type: typing.Literal["multiply"] = "multiply"


class MustBeEmptyAllowedValues(core.ModelBase):
    """The parameter must be omitted or empty."""

    type: typing.Literal["mustBeEmpty"] = "mustBeEmpty"


NearestNeighborsQuery: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["DoubleVector", "NearestNeighborsQueryText"], pydantic.Field(discriminator="type")
]
"""
Queries support either a vector matching the embedding model defined on the property, or text that is 
automatically embedded.
"""


class NearestNeighborsQueryText(core.ModelBase):
    """Automatically embed the text in a vector using the embedding model configured for the given propertyIdentifier."""

    value: str
    type: typing.Literal["text"] = "text"


class NegatePropertyExpression(core.ModelBase):
    """Negates a numeric value."""

    property: DerivedPropertyDefinition
    type: typing.Literal["negate"] = "negate"


NestedInterfacePropertyTypeImplementation: typing_extensions.TypeAlias = (
    typing_extensions.Annotated[
        typing.Union[
            "InterfacePropertyStructFieldImplementation",
            "InterfacePropertyStructImplementation",
            "InterfacePropertyLocalPropertyImplementation",
        ],
        pydantic.Field(discriminator="type"),
    ]
)
"""
Describes how an object type implements an interface property when a reducer is applied to it. Is missing a
reduced property implementation to prevent arbitrarily nested implementations.
"""


class NestedQueryAggregation(core.ModelBase):
    """NestedQueryAggregation"""

    key: typing.Any
    groups: typing.List[QueryAggregation]


class NoLoadLevel(core.ModelBase):
    """
    Returns the property as-is, without applying reducers or extracting a struct main value. Useful as an
    explicit per-property load level (via `PropertyWithLoadLevelSelector`) to opt a property out of a
    `defaultLoadLevel`.
    """

    type: typing.Literal["noLoadLevel"] = "noLoadLevel"


class NotQueryV2(core.ModelBase):
    """Returns objects where the query is not satisfied."""

    value: SearchJsonQueryV2
    type: typing.Literal["not"] = "not"


class NowDatetimeValue(core.ModelBase):
    """The current evaluation time itself. Carries no fields."""

    type: typing.Literal["now"] = "now"


class NumberFormatAffix(core.ModelBase):
    """
    Attach arbitrary text before and/or after the formatted number.
    Example: prefix "USD " and postfix " total" displays as "USD 1,234.56 total"
    """

    base_format_options: NumberFormatOptions = pydantic.Field(alias=str("baseFormatOptions"))  # type: ignore[literal-required]
    affix: Affix
    type: typing.Literal["affix"] = "affix"


class NumberFormatCurrency(core.ModelBase):
    """
    Format numbers as currency values with proper symbols and styling.
    Example: 1234.56 with currency "USD" displays as "USD 1,234.56" (standard) or "USD 1.2K" (compact)
    """

    base_format_options: NumberFormatOptions = pydantic.Field(alias=str("baseFormatOptions"))  # type: ignore[literal-required]
    style: NumberFormatCurrencyStyle
    currency_code: PropertyTypeReferenceOrStringConstant = pydantic.Field(alias=str("currencyCode"))  # type: ignore[literal-required]
    type: typing.Literal["currency"] = "currency"


NumberFormatCurrencyStyle: typing_extensions.TypeAlias = typing.Literal["STANDARD", "COMPACT"]
"""
Currency rendering style options:
- STANDARD: Full currency formatting (e.g., "USD 1,234.56")
- COMPACT: Abbreviated currency formatting (e.g., "USD 1.2K")
"""


class NumberFormatCustomUnit(core.ModelBase):
    """
    Format numbers with custom units not supported by standard formatting.
    Use this for domain-specific units like "requests/sec", "widgets", etc.
    Example: 1500 with unit "widgets" displays as "1,500 widgets"
    """

    base_format_options: NumberFormatOptions = pydantic.Field(alias=str("baseFormatOptions"))  # type: ignore[literal-required]
    unit: PropertyTypeReferenceOrStringConstant
    type: typing.Literal["customUnit"] = "customUnit"


class NumberFormatDuration(core.ModelBase):
    """
    Format numeric values representing time durations.
    - Human readable: 3661 seconds displays as "1h 1m 1s"
    - Timecode: 3661 seconds displays as "01:01:01"
    """

    format_style: DurationFormatStyle = pydantic.Field(alias=str("formatStyle"))  # type: ignore[literal-required]
    precision: typing.Optional[DurationPrecision] = None
    base_value: DurationBaseValue = pydantic.Field(alias=str("baseValue"))  # type: ignore[literal-required]
    type: typing.Literal["duration"] = "duration"


class NumberFormatFixedValues(core.ModelBase):
    """
    Map integer values to custom human-readable strings.
    Example: {1: "First", 2: "Second", 3: "Third"} would display 2 as "Second".
    """

    values: typing.Dict[FixedValuesMapKey, str]
    type: typing.Literal["fixedValues"] = "fixedValues"


NumberFormatNotation: typing_extensions.TypeAlias = typing.Literal[
    "STANDARD", "SCIENTIFIC", "ENGINEERING", "COMPACT"
]
"""
Number notation style options:
- STANDARD: Regular number display ("1,234")
- SCIENTIFIC: Scientific notation ("1.234E3")
- ENGINEERING: Engineering notation ("1.234E3")
- COMPACT: Compact notation ("1.2K")
"""


class NumberFormatOptions(core.ModelBase):
    """
    Base number formatting options that can be applied to all number formatters.
    Controls precision, grouping, rounding, and notation. Consistent with JavaScript's Intl.NumberFormat.

    Examples:
    - useGrouping: true makes 1234567 display as "1,234,567"
    - maximumFractionDigits: 2 makes 3.14159 display as "3.14"
    - notation: SCIENTIFIC makes 1234 display as "1.234E3"
    """

    use_grouping: typing.Optional[bool] = pydantic.Field(alias=str("useGrouping"), default=None)  # type: ignore[literal-required]
    """If true, show a locale-appropriate number grouping (e.g. thousands for en)."""

    convert_negative_to_parenthesis: typing.Optional[bool] = pydantic.Field(alias=str("convertNegativeToParenthesis"), default=None)  # type: ignore[literal-required]
    """If true, wrap negative numbers in parentheses instead of a minus sign."""

    minimum_integer_digits: typing.Optional[int] = pydantic.Field(alias=str("minimumIntegerDigits"), default=None)  # type: ignore[literal-required]
    minimum_fraction_digits: typing.Optional[int] = pydantic.Field(alias=str("minimumFractionDigits"), default=None)  # type: ignore[literal-required]
    maximum_fraction_digits: typing.Optional[int] = pydantic.Field(alias=str("maximumFractionDigits"), default=None)  # type: ignore[literal-required]
    minimum_significant_digits: typing.Optional[int] = pydantic.Field(alias=str("minimumSignificantDigits"), default=None)  # type: ignore[literal-required]
    maximum_significant_digits: typing.Optional[int] = pydantic.Field(alias=str("maximumSignificantDigits"), default=None)  # type: ignore[literal-required]
    notation: typing.Optional[NumberFormatNotation] = None
    rounding_mode: typing.Optional[NumberRoundingMode] = pydantic.Field(alias=str("roundingMode"), default=None)  # type: ignore[literal-required]


class NumberFormatRatio(core.ModelBase):
    """
    Display the value as a ratio with different scaling factors and suffixes:
    - PERCENTAGE: Multiply by 100 and add "%" suffix (0.15 → "15%")
    - PER_MILLE: Multiply by 1000 and add "‰" suffix (0.015 → "15‰")
    - BASIS_POINTS: Multiply by 10000 and add "bps" suffix (0.0015 → "15bps")
    """

    ratio_type: NumberRatioType = pydantic.Field(alias=str("ratioType"))  # type: ignore[literal-required]
    base_format_options: NumberFormatOptions = pydantic.Field(alias=str("baseFormatOptions"))  # type: ignore[literal-required]
    type: typing.Literal["ratio"] = "ratio"


class NumberFormatScale(core.ModelBase):
    """
    Scale the numeric value by dividing by the specified factor and append an appropriate suffix.
    - THOUSANDS: 1500 displays as "1.5K"
    - MILLIONS: 2500000 displays as "2.5M"
    - BILLIONS: 3200000000 displays as "3.2B"
    """

    scale_type: NumberScaleType = pydantic.Field(alias=str("scaleType"))  # type: ignore[literal-required]
    base_format_options: NumberFormatOptions = pydantic.Field(alias=str("baseFormatOptions"))  # type: ignore[literal-required]
    type: typing.Literal["scale"] = "scale"


class NumberFormatStandard(core.ModelBase):
    """
    Standard number formatting with configurable options.
    This provides basic number formatting without any special units, scaling, or transformations.
    """

    base_format_options: NumberFormatOptions = pydantic.Field(alias=str("baseFormatOptions"))  # type: ignore[literal-required]
    type: typing.Literal["standard"] = "standard"


class NumberFormatStandardUnit(core.ModelBase):
    """
    Format numbers with standard units supported by Intl.NumberFormat.
    Examples: "meter", "kilogram", "celsius", "percent"
    Input: 25 with unit "celsius" displays as "25 degrees C"
    """

    base_format_options: NumberFormatOptions = pydantic.Field(alias=str("baseFormatOptions"))  # type: ignore[literal-required]
    unit: PropertyTypeReferenceOrStringConstant
    type: typing.Literal["standardUnit"] = "standardUnit"


NumberRatioType: typing_extensions.TypeAlias = typing.Literal[
    "PERCENTAGE", "PER_MILLE", "BASIS_POINTS"
]
"""
Ratio format options for displaying proportional values:
- PERCENTAGE: Multiply by 100 and add "%" suffix
- PER_MILLE: Multiply by 1000 and add "‰" suffix
- BASIS_POINTS: Multiply by 10000 and add "bps" suffix
"""


NumberRoundingMode: typing_extensions.TypeAlias = typing.Literal["CEIL", "FLOOR", "ROUND_CLOSEST"]
"""
Number rounding behavior:
- CEIL: Always round up (3.1 becomes 4)
- FLOOR: Always round down (3.9 becomes 3)
- ROUND_CLOSEST: Round to nearest (3.4 becomes 3, 3.6 becomes 4)
"""


NumberScaleType: typing_extensions.TypeAlias = typing.Literal["THOUSANDS", "MILLIONS", "BILLIONS"]
"""
Scale factor options for large numbers:
- THOUSANDS: Divide by 1,000 and add "K" suffix
- MILLIONS: Divide by 1,000,000 and add "M" suffix
- BILLIONS: Divide by 1,000,000,000 and add "B" suffix
"""


ObjectEdit: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["ModifyObject", "DeleteObject", "AddObject", "DeleteLink", "AddLink"],
    pydantic.Field(discriminator="type"),
]
"""ObjectEdit"""


class ObjectEditHistoryEntry(core.ModelBase):
    """
    Represents a single object edit operation in the history. This captures when an object was
    created, modified, or deleted as part of an action execution.
    """

    object_primary_key: ObjectPrimaryKeyV2 = pydantic.Field(alias=str("objectPrimaryKey"))  # type: ignore[literal-required]
    operation_id: ActionRid = pydantic.Field(alias=str("operationId"))  # type: ignore[literal-required]
    action_type_rid: ActionTypeRid = pydantic.Field(alias=str("actionTypeRid"))  # type: ignore[literal-required]
    user_id: str = pydantic.Field(alias=str("userId"))  # type: ignore[literal-required]
    """The user ID or principal that performed the action"""

    timestamp: core.AwareDatetime
    """When this edit occurred (ISO 8601 format)"""

    edit: EditHistoryEdit


class ObjectEdits(core.ModelBase):
    """ObjectEdits"""

    edits: typing.List[ObjectEdit]
    added_object_count: int = pydantic.Field(alias=str("addedObjectCount"))  # type: ignore[literal-required]
    modified_objects_count: int = pydantic.Field(alias=str("modifiedObjectsCount"))  # type: ignore[literal-required]
    deleted_objects_count: int = pydantic.Field(alias=str("deletedObjectsCount"))  # type: ignore[literal-required]
    added_links_count: int = pydantic.Field(alias=str("addedLinksCount"))  # type: ignore[literal-required]
    deleted_links_count: int = pydantic.Field(alias=str("deletedLinksCount"))  # type: ignore[literal-required]
    type: typing.Literal["edits"] = "edits"


class ObjectLoadingResponseOptions(core.ModelBase):
    """Optional features to toggle when generating the object loading response."""

    should_load_object_rids: typing.Optional[bool] = pydantic.Field(alias=str("shouldLoadObjectRids"), default=None)  # type: ignore[literal-required]
    """
    Whether returned objects should include their RIDs. Note that resolving object RIDs can add 100s of 
    milliseconds to the execution time, and should be avoided where not needed.
    """


class ObjectLocator(core.ModelBase):
    """An object identifier containing an object type API name and primary key."""

    object_type_api_name: ObjectTypeApiName = pydantic.Field(alias=str("objectTypeApiName"))  # type: ignore[literal-required]
    primary_key_value: PrimaryKeyValue = pydantic.Field(alias=str("primaryKeyValue"))  # type: ignore[literal-required]


class ObjectParameterPropertyArgument(core.ModelBase):
    """Represents an object parameter property argument in a logic rule."""

    parameter_id: ParameterId = pydantic.Field(alias=str("parameterId"))  # type: ignore[literal-required]
    property_type_api_name: PropertyTypeApiName = pydantic.Field(alias=str("propertyTypeApiName"))  # type: ignore[literal-required]
    type: typing.Literal["objectParameterPropertyValue"] = "objectParameterPropertyValue"


ObjectPrimaryKey: typing_extensions.TypeAlias = typing.Dict["PropertyApiName", "PropertyValue"]
"""ObjectPrimaryKey"""


ObjectPrimaryKeyV2: typing_extensions.TypeAlias = typing.Dict[
    "PropertyApiName", "PrimaryKeyValueV2"
]
"""ObjectPrimaryKeyV2"""


ObjectPropertyType: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        core_models.DateType,
        "StructType",
        core_models.StringType,
        core_models.ByteType,
        core_models.DoubleType,
        core_models.GeoPointType,
        core_models.GeotimeSeriesReferenceType,
        core_models.IntegerType,
        core_models.FloatType,
        core_models.GeoShapeType,
        core_models.LongType,
        core_models.BooleanType,
        core_models.CipherTextType,
        core_models.MarkingType,
        core_models.AttachmentType,
        core_models.MediaReferenceType,
        core_models.TimeseriesType,
        "OntologyObjectArrayType",
        core_models.ShortType,
        core_models.VectorType,
        core_models.DecimalType,
        core_models.TimestampType,
    ],
    pydantic.Field(discriminator="type"),
]
"""A union of all the types supported by Ontology Object properties."""


class ObjectPropertyValueConstraint(core.ModelBase):
    """The parameter value must be a property value of an object found within an object set."""

    type: typing.Literal["objectPropertyValue"] = "objectPropertyValue"


class ObjectQueryResultConstraint(core.ModelBase):
    """The parameter value must be the primary key of an object found within an object set."""

    type: typing.Literal["objectQueryResult"] = "objectQueryResult"


ObjectRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of an object, useful for interacting with other Foundry APIs."""


ObjectSet: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "ObjectSetSearchAroundType",
        "ObjectSetStaticType",
        "ObjectSetIntersectionType",
        "ObjectSetWithPropertiesType",
        "ObjectSetInterfaceLinkSearchAroundType",
        "ObjectSetSubtractType",
        "ObjectSetNearestNeighborsType",
        "ObjectSetUnionType",
        "ObjectSetAsTypeType",
        "ObjectSetMethodInputType",
        "ObjectSetReferenceType",
        "ObjectSetFilterType",
        "ObjectSetInterfaceBaseType",
        "ObjectSetAsBaseObjectTypesType",
        "ObjectSetBaseType",
    ],
    pydantic.Field(discriminator="type"),
]
"""Represents the definition of an `ObjectSet` in the `Ontology`."""


class ObjectSetAsBaseObjectTypesType(core.ModelBase):
    """
    Casts the objects in the object set to their base type and thus ensures objects are returned with all of their
    properties in the resulting object set, not just the properties that implement interface properties.
    """

    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    type: typing.Literal["asBaseObjectTypes"] = "asBaseObjectTypes"


class ObjectSetAsTypeType(core.ModelBase):
    """
    Casts an object set to a specified object type or interface type API name. Any object whose object type does
    not match the object type provided or implement the interface type provided will be dropped from the resulting
    object set.
    """

    entity_type: str = pydantic.Field(alias=str("entityType"))  # type: ignore[literal-required]
    """An object type or interface type API name."""

    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    type: typing.Literal["asType"] = "asType"


class ObjectSetBaseType(core.ModelBase):
    """ObjectSetBaseType"""

    object_type: str = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]
    """The API name of the object type."""

    type: typing.Literal["base"] = "base"


class ObjectSetFilterType(core.ModelBase):
    """ObjectSetFilterType"""

    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    where: SearchJsonQueryV2
    type: typing.Literal["filter"] = "filter"


class ObjectSetInterfaceBaseType(core.ModelBase):
    """ObjectSetInterfaceBaseType"""

    interface_type: str = pydantic.Field(alias=str("interfaceType"))  # type: ignore[literal-required]
    """
    An object set with objects that implement the interface with the given interface API name. The objects in 
    the object set will only have properties that implement properties of the given interface, unless you set the includeAllBaseObjectProperties flag.
    """

    include_all_base_object_properties: typing.Optional[bool] = pydantic.Field(alias=str("includeAllBaseObjectProperties"), default=None)  # type: ignore[literal-required]
    """
    A flag that will return all of the underlying object properties for the objects that implement the interface. 
    This includes properties that don't explicitly implement an SPT on the interface.
    """

    type: typing.Literal["interfaceBase"] = "interfaceBase"


class ObjectSetInterfaceLinkSearchAroundType(core.ModelBase):
    """ObjectSetInterfaceLinkSearchAroundType"""

    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    interface_link: InterfaceLinkTypeApiName = pydantic.Field(alias=str("interfaceLink"))  # type: ignore[literal-required]
    type: typing.Literal["interfaceLinkSearchAround"] = "interfaceLinkSearchAround"


class ObjectSetIntersectionType(core.ModelBase):
    """ObjectSetIntersectionType"""

    object_sets: typing.List[ObjectSet] = pydantic.Field(alias=str("objectSets"))  # type: ignore[literal-required]
    type: typing.Literal["intersect"] = "intersect"


class ObjectSetMethodInputType(core.ModelBase):
    """
    ObjectSet which is the root of a MethodObjectSet definition.

    This feature is experimental and not yet generally available.
    """

    type: typing.Literal["methodInput"] = "methodInput"


class ObjectSetNearestNeighborsType(core.ModelBase):
    """
    ObjectSet containing the top `numNeighbors` objects with `propertyIdentifier` nearest to the input vector or
    text. This can only be performed on a property with type vector that has been configured to be searched with
    approximate nearest neighbors using a similarity function configured in the Ontology.

    A non-zero score for each resulting object is returned when the `orderType` in the `orderBy` field is set to
    `relevance`. Note that:
      - Scores will not be returned if a nearestNeighbors object set is composed through union, subtraction
        or intersection with non-nearestNeighbors object sets.
      - If results have scores, the order of the scores will be decreasing (duplicate scores are possible).
    """

    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    property_identifier: PropertyIdentifier = pydantic.Field(alias=str("propertyIdentifier"))  # type: ignore[literal-required]
    num_neighbors: int = pydantic.Field(alias=str("numNeighbors"))  # type: ignore[literal-required]
    """
    The number of objects to return. If the number of documents in the objectType is less than the provided
    value, all objects will be returned. This value is limited to 1 &lt;= numNeighbors &lt;= 500.
    """

    similarity_threshold: typing.Optional[float] = pydantic.Field(alias=str("similarityThreshold"), default=None)  # type: ignore[literal-required]
    """
    The similarity threshold results must be above to be included in the returned in the object set.
    0 &lt;= Threshold &lt;= 1. Where 1 is identical and 0 is least similar.
    """

    query: NearestNeighborsQuery
    type: typing.Literal["nearestNeighbors"] = "nearestNeighbors"


class ObjectSetReferenceType(core.ModelBase):
    """ObjectSetReferenceType"""

    reference: ObjectSetRid
    type: typing.Literal["reference"] = "reference"


ObjectSetRid: typing_extensions.TypeAlias = core.RID
"""ObjectSetRid"""


class ObjectSetSearchAroundType(core.ModelBase):
    """ObjectSetSearchAroundType"""

    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    link: LinkTypeApiName
    type: typing.Literal["searchAround"] = "searchAround"


class ObjectSetStaticType(core.ModelBase):
    """ObjectSetStaticType"""

    objects: typing.List[ObjectRid]
    type: typing.Literal["static"] = "static"


class ObjectSetStreamSubscribeRequest(core.ModelBase):
    """
    `branch` identifies the Foundry branch. `scenarioRid` identifies the Ontology Scenario.
    If a scenario is based on a non-default branch, `branch` must identify that non-default base branch.
    """

    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    branch: typing.Optional[core_models.FoundryBranch] = None
    scenario_rid: typing.Optional[OntologyScenarioRid] = pydantic.Field(alias=str("scenarioRid"), default=None)  # type: ignore[literal-required]
    property_set: typing.List[SelectedPropertyApiName] = pydantic.Field(alias=str("propertySet"))  # type: ignore[literal-required]
    reference_set: typing.List[SelectedPropertyApiName] = pydantic.Field(alias=str("referenceSet"))  # type: ignore[literal-required]
    object_loading_response_options: typing.Optional[ObjectLoadingResponseOptions] = pydantic.Field(alias=str("objectLoadingResponseOptions"), default=None)  # type: ignore[literal-required]


class ObjectSetStreamSubscribeRequests(core.ModelBase):
    """
    The list of object sets that should be subscribed to. A client can stop subscribing to an object set
    by removing the request from subsequent ObjectSetStreamSubscribeRequests.
    """

    id: RequestId
    requests: typing.List[ObjectSetStreamSubscribeRequest]


ObjectSetSubscribeResponse: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["QosError", "SubscriptionSuccess", "SubscriptionError"],
    pydantic.Field(discriminator="type"),
]
"""ObjectSetSubscribeResponse"""


class ObjectSetSubscribeResponses(core.ModelBase):
    """Returns a response for every request in the same order. Duplicate requests will be assigned the same SubscriberId."""

    responses: typing.List[ObjectSetSubscribeResponse]
    id: RequestId
    type: typing.Literal["subscribeResponses"] = "subscribeResponses"


class ObjectSetSubtractType(core.ModelBase):
    """ObjectSetSubtractType"""

    object_sets: typing.List[ObjectSet] = pydantic.Field(alias=str("objectSets"))  # type: ignore[literal-required]
    type: typing.Literal["subtract"] = "subtract"


class ObjectSetUnionType(core.ModelBase):
    """ObjectSetUnionType"""

    object_sets: typing.List[ObjectSet] = pydantic.Field(alias=str("objectSets"))  # type: ignore[literal-required]
    type: typing.Literal["union"] = "union"


ObjectSetUpdate: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["ReferenceUpdate", "ObjectUpdate"], pydantic.Field(discriminator="type")
]
"""ObjectSetUpdate"""


class ObjectSetUpdates(core.ModelBase):
    """ObjectSetUpdates"""

    id: SubscriptionId
    updates: typing.List[ObjectSetUpdate]
    type: typing.Literal["objectSetChanged"] = "objectSetChanged"


class ObjectSetWithPropertiesType(core.ModelBase):
    """
    ObjectSet which returns objects with additional derived properties.

    This feature is experimental and not yet generally available.
    """

    object_set: ObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    derived_properties: typing.Dict[DerivedPropertyApiName, DerivedPropertyDefinition] = pydantic.Field(alias=str("derivedProperties"))  # type: ignore[literal-required]
    """Map of the name of the derived property to return and its definition"""

    type: typing.Literal["withProperties"] = "withProperties"


ObjectState: typing_extensions.TypeAlias = typing.Literal["ADDED_OR_UPDATED", "REMOVED"]
"""
Represents the state of the object within the object set. ADDED_OR_UPDATED indicates that the object was 
added to the set or the object has updated and was previously in the set. REMOVED indicates that the object 
was removed from the set due to the object being deleted or the object no longer meets the object set 
definition.
"""


ObjectTypeApiName: typing_extensions.TypeAlias = str
"""
The name of the object type in the API in camelCase format. To find the API name for your Object Type, use the
`List object types` endpoint or check the **Ontology Manager**.
"""


class ObjectTypeDatasetDatasource(core.ModelBase):
    """An object type datasource backed by a Foundry dataset."""

    dataset_rid: datasets_models.DatasetRid = pydantic.Field(alias=str("datasetRid"))  # type: ignore[literal-required]
    branch: typing.Optional[DatasourceBranchId] = None
    property_mapping: typing.Dict[PropertyApiName, PropertyTypeMappingInfo] = pydantic.Field(alias=str("propertyMapping"))  # type: ignore[literal-required]
    """
    A mapping from property API name to a description of how that property is bound to the dataset. Properties
    whose mapping info cannot be modeled are omitted.
    """

    type: typing.Literal["dataset"] = "dataset"


class ObjectTypeDatasource(core.ModelBase):
    """
    A datasource that supplies property values for an object type. Each object type can have one or more
    datasources; together they back all of the object type's properties. The `definition` carries the RID of the
    backing Foundry resource (for example, the dataset RID for a dataset-backed object type), enabling callers to
    navigate from an object type to its backing data.
    """

    rid: DatasourceRid
    definition: ObjectTypeDatasourceDefinition


ObjectTypeDatasourceDefinition: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "ObjectTypeTimeSeriesDatasource",
        "ObjectTypeUnsupportedDatasource",
        "ObjectTypeRestrictedViewDatasource",
        "ObjectTypeStreamDatasource",
        "ObjectTypeMediaSetViewDatasource",
        "ObjectTypeDirectDatasource",
        "ObjectTypeGeotimeSeriesDatasource",
        "ObjectTypeEditsOnlyDatasource",
        "ObjectTypeDatasetDatasource",
        "ObjectTypeTableDatasource",
    ],
    pydantic.Field(discriminator="type"),
]
"""
The definition of an object type datasource, identifying the kind of Foundry resource that backs the object
type.
"""


class ObjectTypeDirectDatasource(core.ModelBase):
    """
    An object type datasource backed by a direct-write source. Property values are written directly to the
    datasource rather than being read from a separate Foundry resource. Unlike an edits-only datasource, a direct
    datasource has a backing source that values are written to by some writer. An edits-only datasource has no
    backing source at all and its properties are populated solely via Actions.
    """

    direct_source_rid: DirectSourceRid = pydantic.Field(alias=str("directSourceRid"))  # type: ignore[literal-required]
    property_mapping: typing.Dict[PropertyApiName, PropertyTypeMappingInfo] = pydantic.Field(alias=str("propertyMapping"))  # type: ignore[literal-required]
    """
    A mapping from property API name to a description of how that property is bound to the direct datasource.
    Properties whose mapping info cannot be modeled are omitted.
    """

    type: typing.Literal["direct"] = "direct"


class ObjectTypeEdits(core.ModelBase):
    """ObjectTypeEdits"""

    edited_object_types: typing.List[ObjectTypeApiName] = pydantic.Field(alias=str("editedObjectTypes"))  # type: ignore[literal-required]
    type: typing.Literal["largeScaleEdits"] = "largeScaleEdits"


class ObjectTypeEditsHistoryRequest(core.ModelBase):
    """
    Request object for querying object type edits history, containing both filters and pagination parameters

    If objectPrimaryKey property is set, the method will return edits history for the particular object.
    Otherwise, the method will return edits history for all objects of this object type.
    """

    object_primary_key: typing.Optional[ObjectPrimaryKeyV2] = pydantic.Field(alias=str("objectPrimaryKey"), default=None)  # type: ignore[literal-required]
    filters: typing.Optional[EditsHistoryFilter] = None
    sort_order: typing.Optional[EditsHistorySortOrder] = pydantic.Field(alias=str("sortOrder"), default=None)  # type: ignore[literal-required]
    include_all_previous_properties: typing.Optional[bool] = pydantic.Field(alias=str("includeAllPreviousProperties"), default=None)  # type: ignore[literal-required]
    page_size: typing.Optional[int] = pydantic.Field(alias=str("pageSize"), default=None)  # type: ignore[literal-required]
    """The maximum number of edits to return per page. Defaults to 100."""

    page_token: typing.Optional[str] = pydantic.Field(alias=str("pageToken"), default=None)  # type: ignore[literal-required]
    """Token for retrieving the next page of results"""


class ObjectTypeEditsHistoryResponse(core.ModelBase):
    """
    Response containing the history of edits for objects of a specific object type.
    Only contains object edits (create, modify, delete) - link edits are not included.
    """

    data: typing.List[ObjectEditHistoryEntry]
    """List of historical edits for this object type"""

    total_count: typing.Optional[int] = pydantic.Field(alias=str("totalCount"), default=None)  # type: ignore[literal-required]
    """Count of items in the data array above"""

    next_page_token: typing.Optional[str] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    """Token for retrieving the next page of results"""


class ObjectTypeEditsOnlyDatasource(core.ModelBase):
    """
    An object type datasource that is not backed by any external Foundry resource. All properties on the object type
    can only be populated via Actions. Other datasources have edit only *properties*, which are permissioned to the
    backing tabular datasource. This datasource has no backing tabular datasource and is a true edit only object
    type. Note that this datasource type is incompatible with any other datasource and all the properties on the
    object type are backed by it.
    """

    type: typing.Literal["editsOnly"] = "editsOnly"


class ObjectTypeFullMetadata(core.ModelBase):
    """ObjectTypeFullMetadata"""

    object_type: ObjectTypeV2 = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]
    link_types: typing.List[LinkTypeSideV2] = pydantic.Field(alias=str("linkTypes"))  # type: ignore[literal-required]
    implements_interfaces: typing.List[InterfaceTypeApiName] = pydantic.Field(alias=str("implementsInterfaces"))  # type: ignore[literal-required]
    """A list of interfaces that this object type implements."""

    implements_interfaces2: typing.Dict[InterfaceTypeApiName, ObjectTypeInterfaceImplementation] = pydantic.Field(alias=str("implementsInterfaces2"))  # type: ignore[literal-required]
    """A list of interfaces that this object type implements and how it implements them."""

    shared_property_type_mapping: typing.Dict[SharedPropertyTypeApiName, PropertyApiName] = pydantic.Field(alias=str("sharedPropertyTypeMapping"))  # type: ignore[literal-required]
    """
    A map from shared property type API name to backing local property API name for the shared property types 
    present on this object type.
    """


class ObjectTypeGeotimeSeriesDatasource(core.ModelBase):
    """
    An object type datasource backed by a Geotime series integration, providing values for Geotime series reference
    properties.
    """

    geotime_series_integration_rid: GeotimeSeriesIntegrationRid = pydantic.Field(alias=str("geotimeSeriesIntegrationRid"))  # type: ignore[literal-required]
    properties: typing.List[PropertyApiName]
    """The set of properties that are bound to the Geotime series."""

    type: typing.Literal["geotimeSeries"] = "geotimeSeries"


ObjectTypeId: typing_extensions.TypeAlias = str
"""The unique identifier (ID) for an object type. This can be viewed in [Ontology Manager](https://palantir.com/docs/foundry/ontology-manager/overview/)."""


class ObjectTypeInterfaceImplementation(core.ModelBase):
    """ObjectTypeInterfaceImplementation"""

    api_name: typing.Optional[InterfaceTypeApiName] = pydantic.Field(alias=str("apiName"), default=None)  # type: ignore[literal-required]
    rid: typing.Optional[InterfaceTypeRid] = None
    properties: typing.Dict[SharedPropertyTypeApiName, PropertyApiName]
    properties_v2: typing.Dict[InterfacePropertyApiName, InterfacePropertyTypeImplementation] = pydantic.Field(alias=str("propertiesV2"))  # type: ignore[literal-required]
    links: typing.Dict[InterfaceLinkTypeApiName, typing.List[LinkTypeApiName]]
    action_types: typing.Dict[InterfaceActionTypeConstraintApiName, ActionTypeApiName] = pydantic.Field(alias=str("actionTypes"))  # type: ignore[literal-required]
    """
    A map from interface action type constraint API name to the API name of the concrete action type on this
    object type that implements it. Action types the caller is not authorized to access are omitted, so this
    map may not cover every action type constraint declared on the interface.
    """


class ObjectTypeLinkTypeApiNameMapping(core.ModelBase):
    """
    Groups link type API names by the object type they're scoped to. Link type API names are only unique within
    an object type, so this pairing is required to identify a link type unambiguously.
    """

    object_type_api_name: ObjectTypeApiName = pydantic.Field(alias=str("objectTypeApiName"))  # type: ignore[literal-required]
    link_types: typing.List[LinkTypeApiName] = pydantic.Field(alias=str("linkTypes"))  # type: ignore[literal-required]
    """The list of link type API names scoped by the object type."""


class ObjectTypeMediaSetViewDatasource(core.ModelBase):
    """An object type datasource backed by a Foundry media set view, providing media for media reference properties."""

    media_set_rid: core_models.MediaSetRid = pydantic.Field(alias=str("mediaSetRid"))  # type: ignore[literal-required]
    media_set_view_rid: core_models.MediaSetViewRid = pydantic.Field(alias=str("mediaSetViewRid"))  # type: ignore[literal-required]
    properties: typing.List[PropertyApiName]
    """The set of properties that are bound to the media view."""

    type: typing.Literal["mediaSetView"] = "mediaSetView"


class ObjectTypeRestrictedViewDatasource(core.ModelBase):
    """An object type datasource backed by a Foundry restricted view."""

    restricted_view_rid: RestrictedViewRid = pydantic.Field(alias=str("restrictedViewRid"))  # type: ignore[literal-required]
    property_mapping: typing.Dict[PropertyApiName, PropertyTypeMappingInfo] = pydantic.Field(alias=str("propertyMapping"))  # type: ignore[literal-required]
    """
    A mapping from property API name to a description of how that property is bound to the restricted view.
    Properties whose mapping info cannot be modeled are omitted.
    """

    type: typing.Literal["restrictedView"] = "restrictedView"


ObjectTypeRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of an object type, useful for interacting with other Foundry APIs."""


class ObjectTypeStreamDatasource(core.ModelBase):
    """An object type datasource backed by a Foundry stream."""

    stream_rid: StreamRid = pydantic.Field(alias=str("streamRid"))  # type: ignore[literal-required]
    branch: typing.Optional[DatasourceBranchId] = None
    property_mapping: typing.Dict[PropertyApiName, PropertyTypeMappingInfo] = pydantic.Field(alias=str("propertyMapping"))  # type: ignore[literal-required]
    """
    A mapping from property API name to a description of how that property is bound to the stream. Properties
    whose mapping info cannot be modeled are omitted.
    """

    type: typing.Literal["stream"] = "stream"


class ObjectTypeTableDatasource(core.ModelBase):
    """An object type datasource backed by a Foundry table."""

    table_rid: TableRid = pydantic.Field(alias=str("tableRid"))  # type: ignore[literal-required]
    branch: typing.Optional[DatasourceBranchId] = None
    property_mapping: typing.Dict[PropertyApiName, PropertyTypeMappingInfo] = pydantic.Field(alias=str("propertyMapping"))  # type: ignore[literal-required]
    """
    A mapping from property API name to a description of how that property is bound to the table. Properties
    whose mapping info cannot be modeled are omitted.
    """

    type: typing.Literal["table"] = "table"


class ObjectTypeTimeSeriesDatasource(core.ModelBase):
    """An object type datasource backed by a time series sync, providing values for time-dependent properties."""

    time_series_sync_rid: TimeseriesSyncRid = pydantic.Field(alias=str("timeSeriesSyncRid"))  # type: ignore[literal-required]
    properties: typing.List[PropertyApiName]
    """The set of properties that are bound to the time series."""

    type: typing.Literal["timeSeries"] = "timeSeries"


class ObjectTypeUnsupportedDatasource(core.ModelBase):
    """
    A datasource of a kind not yet exposed in the public API. The `unsupportedType` discriminator supplies the
    underlying OMS variant so callers can recognize known but unmodelled cases (e.g., derived properties). Variants
    the adapter does not recognise at all are returned with an `"unknown"` discriminator. The `properties` list
    enumerates the property API names this datasource backs. The `properties` will be empty for `"unknown"`
    datasources.
    """

    unsupported_type: str = pydantic.Field(alias=str("unsupportedType"))  # type: ignore[literal-required]
    """
    A short, stable discriminator naming the underlying OMS variant. E.g., `"derivedProperties"` for
    derived-properties datasources or `"unknown"` for variants the adapter does not recognize.
    """

    properties: typing.List[PropertyApiName]
    """The property API names that this datasource backs."""

    type: typing.Literal["unsupported"] = "unsupported"


class ObjectTypeV2(core.ModelBase):
    """Represents an object type in the Ontology."""

    api_name: ObjectTypeApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    display_name: core_models.DisplayName = pydantic.Field(alias=str("displayName"))  # type: ignore[literal-required]
    status: core_models.ReleaseStatus
    description: typing.Optional[str] = None
    """The description of the object type."""

    plural_display_name: str = pydantic.Field(alias=str("pluralDisplayName"))  # type: ignore[literal-required]
    """The plural display name of the object type."""

    icon: Icon
    primary_key: PropertyApiName = pydantic.Field(alias=str("primaryKey"))  # type: ignore[literal-required]
    properties: typing.Dict[PropertyApiName, PropertyV2]
    """A map of the properties of the object type."""

    rid: ObjectTypeRid
    title_property: PropertyApiName = pydantic.Field(alias=str("titleProperty"))  # type: ignore[literal-required]
    visibility: typing.Optional[ObjectTypeVisibility] = None
    aliases: typing.Optional[typing.List[str]] = None
    """
    Alternative names (synonyms) for the object type, usable as search terms. This field is only populated on
    the get-by-RID read paths (e.g. `getObjectTypeV2`); it is always empty on the `listObjectTypesV2` endpoint.
    """

    datasources: typing.Optional[typing.List[ObjectTypeDatasource]] = None
    """
    The datasources backing this object type which the user has access to see. Only populated when the request
    specifies `includeDatasources=true`. This list may be empty if the user doesn't have access to any
    datasources.
    """


ObjectTypeVisibility: typing_extensions.TypeAlias = typing.Literal["NORMAL", "PROMINENT", "HIDDEN"]
"""The suggested visibility of the object type."""


class ObjectUpdate(core.ModelBase):
    """ObjectUpdate"""

    object: OntologyObjectV2
    state: ObjectState
    type: typing.Literal["object"] = "object"


class OneOfAllowedValues(core.ModelBase):
    """The parameter value must be one of a fixed set of labelled options."""

    options: typing.List[ParameterAllowedValueOption]
    """The predefined set of options."""

    other_values_allowed: bool = pydantic.Field(alias=str("otherValuesAllowed"))  # type: ignore[literal-required]
    """Whether values outside `options` are allowed."""

    type: typing.Literal["oneOf"] = "oneOf"


class OneOfConstraint(core.ModelBase):
    """The parameter has a manually predefined set of options."""

    options: typing.List[ParameterOption]
    other_values_allowed: bool = pydantic.Field(alias=str("otherValuesAllowed"))  # type: ignore[literal-required]
    """A flag denoting whether custom, user provided values will be considered valid. This is configured via the **Allowed "Other" value** toggle in the **Ontology Manager**."""

    type: typing.Literal["oneOf"] = "oneOf"


OntologyApiName: typing_extensions.TypeAlias = str
"""OntologyApiName"""


class OntologyArrayType(core.ModelBase):
    """OntologyArrayType"""

    item_type: OntologyDataType = pydantic.Field(alias=str("itemType"))  # type: ignore[literal-required]
    type: typing.Literal["array"] = "array"


class OntologyBaseBranch(core.ModelBase):
    """A branch reference used to initialize a scenario."""

    branch: core_models.FoundryBranch
    type: typing.Literal["branch"] = "branch"


OntologyDataType: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        core_models.DateType,
        "OntologyStructType",
        "OntologySetType",
        core_models.StringType,
        core_models.ByteType,
        core_models.DoubleType,
        core_models.IntegerType,
        core_models.FloatType,
        core_models.AnyType,
        core_models.LongType,
        core_models.BooleanType,
        core_models.CipherTextType,
        core_models.MarkingType,
        core_models.UnsupportedType,
        core_models.MediaReferenceType,
        "OntologyArrayType",
        "OntologyObjectSetType",
        core_models.BinaryType,
        core_models.ShortType,
        core_models.DecimalType,
        "OntologyMapType",
        core_models.TimestampType,
        "OntologyObjectType",
    ],
    pydantic.Field(discriminator="type"),
]
"""A union of all the primitive types used by Palantir's Ontology-based products."""


class OntologyFullMetadata(core.ModelBase):
    """OntologyFullMetadata"""

    ontology: OntologyV2
    object_types: typing.Dict[ObjectTypeApiName, ObjectTypeFullMetadata] = pydantic.Field(alias=str("objectTypes"))  # type: ignore[literal-required]
    action_types: typing.Dict[ActionTypeApiName, ActionTypeV2] = pydantic.Field(alias=str("actionTypes"))  # type: ignore[literal-required]
    action_types_full_metadata: typing.Dict[ActionTypeApiName, ActionTypeFullMetadata] = pydantic.Field(alias=str("actionTypesFullMetadata"))  # type: ignore[literal-required]
    """
    The full metadata for the loaded action types. This is only populated if the request opted in to it by
    setting `includeActionTypeFullMetadata` to `true`. The `actionTypes` field is always populated,
    regardless of this flag.
    """

    query_types: typing.Dict[VersionedQueryTypeApiName, QueryTypeV2] = pydantic.Field(alias=str("queryTypes"))  # type: ignore[literal-required]
    interface_types: typing.Dict[InterfaceTypeApiName, InterfaceType] = pydantic.Field(alias=str("interfaceTypes"))  # type: ignore[literal-required]
    shared_property_types: typing.Dict[SharedPropertyTypeApiName, SharedPropertyType] = pydantic.Field(alias=str("sharedPropertyTypes"))  # type: ignore[literal-required]
    branch: typing.Optional[core_models.BranchMetadata] = None
    value_types: typing.Dict[ValueTypeApiName, OntologyValueType] = pydantic.Field(alias=str("valueTypes"))  # type: ignore[literal-required]


OntologyIdentifier: typing_extensions.TypeAlias = str
"""
The API name or RID of the Ontology. To find the API name or RID, use the **List Ontologies** endpoint or
check the **Ontology Manager**.
"""


class OntologyInterfaceObjectSetType(core.ModelBase):
    """OntologyInterfaceObjectSetType"""

    interface_type_api_name: InterfaceTypeApiName = pydantic.Field(alias=str("interfaceTypeApiName"))  # type: ignore[literal-required]
    type: typing.Literal["interfaceObjectSet"] = "interfaceObjectSet"


class OntologyInterfaceObjectType(core.ModelBase):
    """OntologyInterfaceObjectType"""

    interface_type_api_name: typing.Optional[InterfaceTypeApiName] = pydantic.Field(alias=str("interfaceTypeApiName"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["interfaceObject"] = "interfaceObject"


class OntologyMapType(core.ModelBase):
    """OntologyMapType"""

    key_type: OntologyDataType = pydantic.Field(alias=str("keyType"))  # type: ignore[literal-required]
    value_type: OntologyDataType = pydantic.Field(alias=str("valueType"))  # type: ignore[literal-required]
    type: typing.Literal["map"] = "map"


class OntologyObjectArrayType(core.ModelBase):
    """OntologyObjectArrayType"""

    sub_type: ObjectPropertyType = pydantic.Field(alias=str("subType"))  # type: ignore[literal-required]
    reducers: typing.List[OntologyObjectArrayTypeReducer]
    """
    If non-empty, this property can be reduced to a single value of the subtype. The reducers are applied in
    order to determine a winning value. The array can be loaded as a reduced value or as the full array in an
    object set.
    """

    type: typing.Literal["array"] = "array"


class OntologyObjectArrayTypeReducer(core.ModelBase):
    """OntologyObjectArrayTypeReducer"""

    direction: OntologyObjectArrayTypeReducerSortDirection
    field: typing.Optional[StructFieldApiName] = None


OntologyObjectArrayTypeReducerSortDirection: typing_extensions.TypeAlias = typing.Literal[
    "ASCENDING_NULLS_LAST", "DESCENDING_NULLS_LAST"
]
"""OntologyObjectArrayTypeReducerSortDirection"""


class OntologyObjectSetType(core.ModelBase):
    """OntologyObjectSetType"""

    object_api_name: typing.Optional[ObjectTypeApiName] = pydantic.Field(alias=str("objectApiName"), default=None)  # type: ignore[literal-required]
    object_type_api_name: typing.Optional[ObjectTypeApiName] = pydantic.Field(alias=str("objectTypeApiName"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["objectSet"] = "objectSet"


class OntologyObjectType(core.ModelBase):
    """OntologyObjectType"""

    object_api_name: ObjectTypeApiName = pydantic.Field(alias=str("objectApiName"))  # type: ignore[literal-required]
    object_type_api_name: ObjectTypeApiName = pydantic.Field(alias=str("objectTypeApiName"))  # type: ignore[literal-required]
    type: typing.Literal["object"] = "object"


class OntologyObjectTypeReferenceType(core.ModelBase):
    """OntologyObjectTypeReferenceType"""

    type: typing.Literal["objectType"] = "objectType"


OntologyObjectV2: typing_extensions.TypeAlias = typing.Dict["PropertyApiName", "PropertyValue"]
"""Represents an object in the Ontology."""


OntologyRid: typing_extensions.TypeAlias = core.RID
"""
The unique Resource Identifier (RID) of the Ontology. To look up your Ontology RID, please use the
`List ontologies` endpoint or check the **Ontology Manager**.
"""


OntologyScenarioRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of an ontology scenario."""


class OntologySetType(core.ModelBase):
    """OntologySetType"""

    item_type: OntologyDataType = pydantic.Field(alias=str("itemType"))  # type: ignore[literal-required]
    type: typing.Literal["set"] = "set"


class OntologyStructField(core.ModelBase):
    """OntologyStructField"""

    name: core_models.StructFieldName
    field_type: OntologyDataType = pydantic.Field(alias=str("fieldType"))  # type: ignore[literal-required]
    required: bool


class OntologyStructType(core.ModelBase):
    """OntologyStructType"""

    fields: typing.List[OntologyStructField]
    type: typing.Literal["struct"] = "struct"


OntologyTransactionId: typing_extensions.TypeAlias = str
"""The ID identifying a transaction."""


class OntologyV2(core.ModelBase):
    """Metadata about an Ontology."""

    api_name: OntologyApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    display_name: core_models.DisplayName = pydantic.Field(alias=str("displayName"))  # type: ignore[literal-required]
    description: str
    rid: OntologyRid


class OntologyValueType(core.ModelBase):
    """OntologyValueType"""

    api_name: ValueTypeApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    display_name: core_models.DisplayName = pydantic.Field(alias=str("displayName"))  # type: ignore[literal-required]
    description: typing.Optional[str] = None
    rid: ValueTypeRid
    status: typing.Optional[ValueTypeStatus] = None
    field_type: ValueTypeFieldType = pydantic.Field(alias=str("fieldType"))  # type: ignore[literal-required]
    version: str
    constraints: typing.List[ValueTypeConstraint]


class OrActionTypesQueryV2(core.ModelBase):
    """Returns action types where at least one query is satisfied. An empty list matches no action types."""

    value: typing.List[ActionTypeSearchJsonQueryV2]
    type: typing.Literal["or"] = "or"


class OrQueryV2(core.ModelBase):
    """Returns objects where at least 1 query is satisfied."""

    value: typing.List[SearchJsonQueryV2]
    type: typing.Literal["or"] = "or"


OrderBy: typing_extensions.TypeAlias = str
"""
A command representing the list of properties to order by. Properties should be delimited by commas and
prefixed by `p` or `properties`. The format expected format is
`orderBy=properties.{property}:{sortDirection},properties.{property}:{sortDirection}...`

By default, the ordering for a property is ascending, and this can be explicitly specified by appending 
`:asc` (for ascending) or `:desc` (for descending).

Example: use `orderBy=properties.lastName:asc` to order by a single property, 
`orderBy=properties.lastName,properties.firstName,properties.age:desc` to order by multiple properties. 
You may also use the shorthand `p` instead of `properties` such as `orderBy=p.lastName:asc`.
"""


OrderByDirection: typing_extensions.TypeAlias = typing.Literal["ASC", "DESC"]
"""OrderByDirection"""


class ParameterAllowedValueOption(core.ModelBase):
    """A possible value for the parameter."""

    display_name: core_models.DisplayName = pydantic.Field(alias=str("displayName"))  # type: ignore[literal-required]
    value: DataValue


ParameterAllowedValues: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "OneOfAllowedValues",
        "DatetimeAllowedValues",
        "AttachmentAllowedValues",
        "ValueTypeAllowedValues",
        "MarkdownAllowedValues",
        "RangeAllowedValues",
        "MustBeEmptyAllowedValues",
        "TextAllowedValues",
    ],
    pydantic.Field(discriminator="type"),
]
"""The allowed-values constraint configured on an action parameter."""


class ParameterArraySize(core.ModelBase):
    """Bounds on the size of an array-typed parameter."""

    gte: typing.Optional[int] = None
    """Greater than or equal."""

    lte: typing.Optional[int] = None
    """Less than or equal."""


ParameterConstraintValue: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "StaticConstraintValue", "ParameterLengthConstraintValue", "ParameterValueConstraintValue"
    ],
    pydantic.Field(discriminator="type"),
]
"""
The source of a constraint bound value: either a literal or a reference to another parameter that is
resolved when the action is validated or applied.
"""


ParameterDatetimeValue: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["NowDatetimeValue", "FixedDatetimeValue", "RelativeDatetimeValue"],
    pydantic.Field(discriminator="type"),
]
"""A datetime bound value."""


ParameterEvaluatedConstraint: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "StructEvaluatedConstraint",
        "OneOfConstraint",
        "ArrayEvaluatedConstraint",
        "GroupMemberConstraint",
        "ObjectPropertyValueConstraint",
        "RangeConstraint",
        "ArraySizeConstraint",
        "ObjectQueryResultConstraint",
        "StringLengthConstraint",
        "StringRegexMatchConstraint",
        "UnevaluableConstraint",
    ],
    pydantic.Field(discriminator="type"),
]
"""
A constraint that an action parameter value must satisfy in order to be considered valid.
Constraints can be configured on action parameters in the **Ontology Manager**. 
Applicable constraints are determined dynamically based on parameter inputs. 
Parameter values are evaluated against the final set of constraints.

The type of the constraint.
| Type                  | Description                                                                                                                                                                                                                     |
|-----------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `arraySize`           | The parameter expects an array of values and the size of the array must fall within the defined range.                                                                                                                          |
| `groupMember`         | The parameter value must be the user id of a member belonging to at least one of the groups defined by the constraint.                                                                                                          |
| `objectPropertyValue` | The parameter value must be a property value of an object found within an object set.                                                                                                                                           |
| `objectQueryResult`   | The parameter value must be the primary key of an object found within an object set.                                                                                                                                            |
| `oneOf`               | The parameter has a manually predefined set of options.                                                                                                                                                                         |
| `range`               | The parameter value must be within the defined range.                                                                                                                                                                           |
| `stringLength`        | The parameter value must have a length within the defined range.                                                                                                                                                                |
| `stringRegexMatch`    | The parameter value must match a predefined regular expression.                                                                                                                                                                 |
| `unevaluable`         | The parameter cannot be evaluated because it depends on another parameter or object set that can't be evaluated. This can happen when a parameter's allowed values are defined by another parameter that is missing or invalid. |
"""


class ParameterEvaluationResult(core.ModelBase):
    """Represents the validity of a parameter against the configured constraints."""

    result: ValidationResult
    evaluated_constraints: typing.List[ParameterEvaluatedConstraint] = pydantic.Field(alias=str("evaluatedConstraints"))  # type: ignore[literal-required]
    required: bool
    """Represents whether the parameter is a required input to the action."""

    default_value: typing.Optional[DataValue] = pydantic.Field(alias=str("defaultValue"), default=None)  # type: ignore[literal-required]


ParameterId: typing_extensions.TypeAlias = str
"""
The unique identifier of the parameter. Parameters are used as inputs when an action or query is applied.
Parameters can be viewed and managed in the **Ontology Manager**.
"""


class ParameterIdArgument(core.ModelBase):
    """Represents a parameter ID argument in a logic rule."""

    parameter_id: ParameterId = pydantic.Field(alias=str("parameterId"))  # type: ignore[literal-required]
    type: typing.Literal["parameterId"] = "parameterId"


class ParameterLengthConstraintValue(core.ModelBase):
    """
    The bound is the number of elements in another list parameter's value on the same action type. Clients
    should tolerate a reference that cannot be resolved, as the referenced parameter is not itself guaranteed
    to be surfaced.
    """

    parameter_id: ParameterId = pydantic.Field(alias=str("parameterId"))  # type: ignore[literal-required]
    type: typing.Literal["parameterLength"] = "parameterLength"


class ParameterNameActionTypesQueryV2(core.ModelBase):
    """Returns action types with a parameter whose name matches the given string predicate."""

    value: FullTextStringPredicateV2
    type: typing.Literal["parameterName"] = "parameterName"


class ParameterOption(core.ModelBase):
    """A possible value for the parameter. This is defined in the **Ontology Manager** by Actions admins."""

    display_name: typing.Optional[core_models.DisplayName] = pydantic.Field(alias=str("displayName"), default=None)  # type: ignore[literal-required]
    value: typing.Optional[typing.Any] = None
    """An allowed configured value for a parameter within an action."""


class ParameterRidActionTypesQueryV2(core.ModelBase):
    """Returns action types with a parameter matching the given parameter rid."""

    value: ActionParameterRid
    type: typing.Literal["parameterRid"] = "parameterRid"


class ParameterValueConstraintValue(core.ModelBase):
    """
    The bound is the value of another parameter on the same action type. Clients should tolerate a reference
    that cannot be resolved, as the referenced parameter is not itself guaranteed to be surfaced.
    """

    parameter_id: ParameterId = pydantic.Field(alias=str("parameterId"))  # type: ignore[literal-required]
    type: typing.Literal["parameterValue"] = "parameterValue"


class PermissionModelActionTypesQueryV2(core.ModelBase):
    """Returns action types with the given permission model."""

    value: ActionTypePermissionModelFilter
    type: typing.Literal["permissionModel"] = "permissionModel"


Plaintext: typing_extensions.TypeAlias = str
"""Plaintext"""


class PostTransactionEditsRequest(core.ModelBase):
    """The request payload for staging edits to a transaction."""

    edits: typing.List[TransactionEdit]


class PostTransactionEditsResponse(core.ModelBase):
    """PostTransactionEditsResponse"""


class PreciseDuration(core.ModelBase):
    """A measurement of duration."""

    value: int
    """The duration value."""

    unit: PreciseTimeUnit
    type: typing.Literal["duration"] = "duration"


PreciseTimeUnit: typing_extensions.TypeAlias = typing.Literal[
    "NANOSECONDS", "SECONDS", "MINUTES", "HOURS", "DAYS", "WEEKS"
]
"""The unit of a fixed-width duration. Each day is 24 hours and each week is 7 days."""


class PrefixOnLastTokenRule(core.ModelBase):
    """
    Matches intervals containing all the terms, using exact match for all but the last term, and prefix match for
    the last term. Ordering of the terms in the query is preserved.
    """

    query: str
    type: typing.Literal["prefixOnLastToken"] = "prefixOnLastToken"


class PrimaryKeyPropertySelector(core.ModelBase):
    """Specifies the primary key property of an object type which is present on all object types."""

    type: typing.Literal["primaryKeyProperty"] = "primaryKeyProperty"


PrimaryKeyValue: typing_extensions.TypeAlias = typing.Any
"""Represents the primary key value that is used as a unique identifier for an object."""


PrimaryKeyValueV2: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "DateValue",
        "StringValue",
        "TimestampValue",
        "BooleanValue",
        "IntegerValue",
        "DoubleValue",
        "LongValue",
    ],
    pydantic.Field(discriminator="type"),
]
"""PrimaryKeyValueV2"""


PropertyApiName: typing_extensions.TypeAlias = str
"""
The name of the property in the API. To find the API name for your property, use the `Get object type`
endpoint or check the **Ontology Manager**.
"""


class PropertyApiNameSelector(core.ModelBase):
    """A property api name that references properties to query on."""

    api_name: PropertyApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    type: typing.Literal["property"] = "property"


class PropertyBooleanFormattingRule(core.ModelBase):
    """Formatting configuration for boolean property values."""

    value_if_true: str = pydantic.Field(alias=str("valueIfTrue"))  # type: ignore[literal-required]
    """Value to display if this boolean is true"""

    value_if_false: str = pydantic.Field(alias=str("valueIfFalse"))  # type: ignore[literal-required]
    """Value to display if this boolean is false"""

    type: typing.Literal["boolean"] = "boolean"


class PropertyDateFormattingRule(core.ModelBase):
    """Formatting configuration for date property values."""

    format: DatetimeFormat
    type: typing.Literal["date"] = "date"


PropertyFilter: typing_extensions.TypeAlias = str
"""
Represents a filter used on properties.

Endpoints that accept this supports optional parameters that have the form:
`properties.{propertyApiName}.{propertyFilter}={propertyValueEscapedString}` to filter the returned objects.
For instance, you may use `properties.firstName.eq=John` to find objects that contain a property called
"firstName" that has the exact value of "John".

The following are a list of supported property filters:

- `properties.{propertyApiName}.contains` - supported on arrays and can be used to filter array properties
  that have at least one of the provided values. If multiple query parameters are provided, then objects
  that have any of the given values for the specified property will be matched.
- `properties.{propertyApiName}.eq` - used to filter objects that have the exact value for the provided
  property. If multiple query parameters are provided, then objects that have any of the given values
  will be matched. For instance, if the user provides a request by doing
  `?properties.firstName.eq=John&properties.firstName.eq=Anna`, then objects that have a firstName property
  of either John or Anna will be matched. This filter is supported on all property types except Arrays.
- `properties.{propertyApiName}.neq` - used to filter objects that do not have the provided property values.
  Similar to the `eq` filter, if multiple values are provided, then objects that have any of the given values
  will be excluded from the result.
- `properties.{propertyApiName}.lt`, `properties.{propertyApiName}.lte`, `properties.{propertyApiName}.gt`
  `properties.{propertyApiName}.gte` - represent less than, less than or equal to, greater than, and greater
  than or equal to respectively. These are supported on date, timestamp, byte, integer, long, double, decimal.
- `properties.{propertyApiName}.isNull` - used to filter objects where the provided property is (or is not) null.
  This filter is supported on all property types.
"""


PropertyId: typing_extensions.TypeAlias = str
"""
The immutable ID of a property. Property IDs are only used to identify properties in the **Ontology Manager**
application and assign them API names. In every other case, API names should be used instead of property IDs.
"""


PropertyIdentifier: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "PropertyApiNameSelector",
        "StructFieldSelector",
        "PropertyWithLoadLevelSelector",
        "TitlePropertySelector",
        "PrimaryKeyPropertySelector",
    ],
    pydantic.Field(discriminator="type"),
]
"""An identifier used to select properties or struct fields."""


class PropertyImplementation(core.ModelBase):
    """PropertyImplementation"""

    property_api_name: PropertyApiName = pydantic.Field(alias=str("propertyApiName"))  # type: ignore[literal-required]
    type: typing.Literal["property"] = "property"


class PropertyKnownTypeFormattingRule(core.ModelBase):
    """Formatting configuration for known Foundry types."""

    known_type: KnownType = pydantic.Field(alias=str("knownType"))  # type: ignore[literal-required]
    type: typing.Literal["knownType"] = "knownType"


PropertyLoadLevel: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "ApplyReducersAndExtractMainValueLoadLevel",
        "ApplyReducersLoadLevel",
        "ExtractMainValueLoadLevel",
        "NoLoadLevel",
    ],
    pydantic.Field(discriminator="type"),
]
"""
The load level of the property:
- APPLY_REDUCERS: Returns a single value of an array as configured in the ontology.
- EXTRACT_MAIN_VALUE: Returns the main value of a struct as configured in the ontology.
- APPLY_REDUCERS_AND_EXTRACT_MAIN_VALUE: Performs both to return the reduced main value.
- NO_LOAD_LEVEL: Returns the property as-is, without applying reducers or extracting a struct main value.
"""


class PropertyMarkingSummary(core.ModelBase):
    """All marking requirements applicable to a property value."""

    conjunctive: typing.Optional[ConjunctiveMarkingSummary] = None
    disjunctive: typing.Optional[DisjunctiveMarkingSummary] = None
    container_conjunctive: typing.Optional[ContainerConjunctiveMarkingSummary] = pydantic.Field(alias=str("containerConjunctive"), default=None)  # type: ignore[literal-required]
    container_disjunctive: typing.Optional[ContainerDisjunctiveMarkingSummary] = pydantic.Field(alias=str("containerDisjunctive"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["propertyMarkingSummary"] = "propertyMarkingSummary"


class PropertyNumberFormattingRule(core.ModelBase):
    """Wrapper for numeric formatting options."""

    number_type: PropertyNumberFormattingRuleType = pydantic.Field(alias=str("numberType"))  # type: ignore[literal-required]
    type: typing.Literal["number"] = "number"


PropertyNumberFormattingRuleType: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "NumberFormatStandard",
        "NumberFormatDuration",
        "NumberFormatFixedValues",
        "NumberFormatAffix",
        "NumberFormatScale",
        "NumberFormatCurrency",
        "NumberFormatStandardUnit",
        "NumberFormatCustomUnit",
        "NumberFormatRatio",
    ],
    pydantic.Field(discriminator="type"),
]
"""PropertyNumberFormattingRuleType"""


PropertyOrStructFieldOfPropertyImplementation: typing_extensions.TypeAlias = (
    typing_extensions.Annotated[
        typing.Union["StructFieldOfPropertyImplementation", "PropertyImplementation"],
        pydantic.Field(discriminator="type"),
    ]
)
"""PropertyOrStructFieldOfPropertyImplementation"""


class PropertySecurities(core.ModelBase):
    """A disjunctive set of security results for a property value."""

    disjunction: typing.List[PropertySecurity]


PropertySecurity: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["PropertyMarkingSummary", "UnsupportedPolicy", "ErrorComputingSecurity"],
    pydantic.Field(discriminator="type"),
]
"""PropertySecurity"""


class PropertyTimestampFormattingRule(core.ModelBase):
    """Formatting configuration for timestamp property values."""

    format: DatetimeFormat
    display_timezone: DatetimeTimezone = pydantic.Field(alias=str("displayTimezone"))  # type: ignore[literal-required]
    type: typing.Literal["timestamp"] = "timestamp"


PropertyTypeApiName: typing_extensions.TypeAlias = str
"""PropertyTypeApiName"""


PropertyTypeMappingInfo: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["StructPropertyMapping", "ColumnPropertyMapping", "EditOnlyPropertyMapping"],
    pydantic.Field(discriminator="type"),
]
"""
Describes how a single object type property is bound to its backing tabular datasource. A property may be backed
by a single column, by a struct (with nested field mappings), or be edit-only (no backing column even though it
is permissioned to the tabular datasource).
"""


class PropertyTypeReference(core.ModelBase):
    """PropertyTypeReference"""

    property_api_name: str = pydantic.Field(alias=str("propertyApiName"))  # type: ignore[literal-required]
    """The API name of the PropertyType"""

    type: typing.Literal["propertyType"] = "propertyType"


PropertyTypeReferenceOrStringConstant: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["StringConstant", "PropertyTypeReference"], pydantic.Field(discriminator="type")
]
"""PropertyTypeReferenceOrStringConstant"""


PropertyTypeRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of a property."""


PropertyTypeStatus: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "DeprecatedPropertyTypeStatus",
        "ActivePropertyTypeStatus",
        "ExperimentalPropertyTypeStatus",
        "ExamplePropertyTypeStatus",
    ],
    pydantic.Field(discriminator="type"),
]
"""The status to indicate whether the PropertyType is either Experimental, Active, Deprecated, or Example."""


PropertyTypeVisibility: typing_extensions.TypeAlias = typing.Literal[
    "NORMAL", "PROMINENT", "HIDDEN"
]
"""PropertyTypeVisibility"""


class PropertyV2(core.ModelBase):
    """Details about some property of an object."""

    description: typing.Optional[str] = None
    display_name: typing.Optional[core_models.DisplayName] = pydantic.Field(alias=str("displayName"), default=None)  # type: ignore[literal-required]
    data_type: ObjectPropertyType = pydantic.Field(alias=str("dataType"))  # type: ignore[literal-required]
    rid: PropertyTypeRid
    status: typing.Optional[PropertyTypeStatus] = None
    visibility: typing.Optional[PropertyTypeVisibility] = None
    value_type_api_name: typing.Optional[ValueTypeApiName] = pydantic.Field(alias=str("valueTypeApiName"), default=None)  # type: ignore[literal-required]
    value_formatting: typing.Optional[PropertyValueFormattingRule] = pydantic.Field(alias=str("valueFormatting"), default=None)  # type: ignore[literal-required]
    type_classes: typing.Optional[typing.List[TypeClass]] = pydantic.Field(alias=str("typeClasses"), default=None)  # type: ignore[literal-required]


PropertyValue: typing_extensions.TypeAlias = typing.Any
"""
Represents the value of a property in the following format.

| Type                                                                                                                      | JSON encoding                                               | Example                                                                                            |
|---------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| Array                                                                                                                     | array                                                       | `["alpha", "bravo", "charlie"]`                                                                    |
| [Attachment](https://palantir.com/docs/foundry/api/v2/ontologies-v2-resources/attachment-properties/attachment-property-basics/)              | JSON encoded `AttachmentProperty` object                    | `{"rid":"ri.blobster.main.attachment.2f944bae-5851-4204-8615-920c969a9f2e"}`                       |
| Boolean                                                                                                                   | boolean                                                     | `true`                                                                                             |
| Byte                                                                                                                      | number                                                      | `31`                                                                                               |
| CipherText                                                                                                                | string                                                      | `"CIPHER::ri.bellaso.main.cipher-channel.e414ab9e-b606-499a-a0e1-844fa296ba7e::unzjs3VifsTxuIpf1fH1CJ7OaPBr2bzMMdozPaZJtCii8vVG60yXIEmzoOJaEl9mfFFe::CIPHER"`                                                                                                                                                                                        |        
| Date                                                                                                                      | ISO 8601 extended local date string                         | `"2021-05-01"`                                                                                     |
| Decimal                                                                                                                   | string                                                      | `"2.718281828"`                                                                                    |
| Double                                                                                                                    | number                                                      | `3.14159265`                                                                                       |
| Float                                                                                                                     | number                                                      | `3.14159265`                                                                                       |
| GeoPoint                                                                                                                  | geojson                                                     | `{"type":"Point","coordinates":[102.0,0.5]}`                                                       |
| GeoShape                                                                                                                  | geojson                                                     | `{"type":"LineString","coordinates":[[102.0,0.0],[103.0,1.0],[104.0,0.0],[105.0,1.0]]}`            |
| Integer                                                                                                                   | number                                                      | `238940`                                                                                           |
| Long                                                                                                                      | string                                                      | `"58319870951433"`                                                                                 |
| [MediaReference](https://palantir.com/docs/foundry/api/v2/ontologies-v2-resources/media-reference-properties/media-reference-property-basics/)| JSON encoded `MediaReference` object                        | `{"mimeType":"application/pdf","reference":{"type":"mediaSetViewItem","mediaSetViewItem":{"mediaSetRid":"ri.mio.main.media-set.4153d42f-ca4b-4e42-8ca5-8e6aa7edb642","mediaSetViewRid":"ri.mio.main.view.82a798ad-d637-4595-acc6-987bcf16629b","mediaItemRid":"ri.mio.main.media-item.001ec98b-1620-4814-9e17-8e9c4e536225"}}}`                       |
| Secured Property Value                                                                                                    | JSON encoded `SecuredPropertyValue` object                  | `{"value": 10, "propertySecurityIndex" : 5}`                                                       |
| Short                                                                                                                     | number                                                      | `8739`                                                                                             |
| String                                                                                                                    | string                                                      | `"Call me Ishmael"`                                                                                |
| Struct                                                                                                                    | JSON object of struct field API name -> value               | {"firstName": "Alex", "lastName": "Karp"}                                                          |
| Timestamp                                                                                                                 | ISO 8601 extended offset date-time string in UTC zone       | `"2021-01-04T05:00:00Z"`                                                                           |
| [Timeseries](https://palantir.com/docs/foundry/api/v2/ontologies-v2-resources/time-series-properties/time-series-property-basics/)            | JSON encoded `TimeseriesProperty` object or seriesId string | `{"seriesId": "wellPressureSeriesId", "syncRid": ri.time-series-catalog.main.sync.04f5ac1f-91bf-44f9-a51f-4f34e06e42df"}` or `{"templateRid": "ri.codex-emu.main.template.367cac64-e53b-4653-b111-f61856a63df9", "templateVersion": "0.0.0"}` or `"wellPressureSeriesId"`|                                                                           |
| Vector                                                                                                                    | array                                                       | `[0.1, 0.3, 0.02, 0.05 , 0.8, 0.4]`                                                                |

Note that for backwards compatibility, the Boolean, Byte, Double, Float, Integer, and Short types can also be encoded as JSON strings.
"""


PropertyValueEscapedString: typing_extensions.TypeAlias = str
"""Represents the value of a property in string format. This is used in URL parameters."""


PropertyValueFormattingRule: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "PropertyDateFormattingRule",
        "PropertyNumberFormattingRule",
        "PropertyBooleanFormattingRule",
        "PropertyKnownTypeFormattingRule",
        "PropertyTimestampFormattingRule",
    ],
    pydantic.Field(discriminator="type"),
]
"""
This feature is experimental and may change in a future release.
Comprehensive formatting configuration for displaying property values in user interfaces. 
Supports different value types including numbers, dates, timestamps, booleans, and known Foundry types.

Each formatter type provides specific options tailored to that data type:
- Numbers: Support for percentages, currencies, units, scaling, and custom formatting
- Dates/Timestamps: Localized and custom formatting patterns
- Booleans: Custom true/false display text
- Known types: Special formatting for Foundry-specific identifiers
"""


class PropertyWithLoadLevelSelector(core.ModelBase):
    """
    A combination of a property identifier and the load level to apply to the property. You can select a reduced
    value for arrays and the main value for structs. If the provided load level cannot be applied to the property
    type, then it will be ignored. This selector is experimental and may not work in filters or sorts.
    """

    property_identifier: PropertyIdentifier = pydantic.Field(alias=str("propertyIdentifier"))  # type: ignore[literal-required]
    load_level: PropertyLoadLevel = pydantic.Field(alias=str("loadLevel"))  # type: ignore[literal-required]
    type: typing.Literal["propertyWithLoadLevel"] = "propertyWithLoadLevel"


class QosError(core.ModelBase):
    """An error indicating that the subscribe request should be attempted on a different node."""

    type: typing.Literal["qos"] = "qos"


class QueryAggregation(core.ModelBase):
    """QueryAggregation"""

    key: typing.Any
    value: typing.Any


QueryAggregationKeyType: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        core_models.DateType,
        core_models.BooleanType,
        core_models.StringType,
        core_models.DoubleType,
        "QueryAggregationRangeType",
        core_models.IntegerType,
        core_models.TimestampType,
    ],
    pydantic.Field(discriminator="type"),
]
"""A union of all the types supported by query aggregation keys."""


QueryAggregationRangeSubType: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        core_models.DateType,
        core_models.DoubleType,
        core_models.IntegerType,
        core_models.TimestampType,
    ],
    pydantic.Field(discriminator="type"),
]
"""A union of all the types supported by query aggregation ranges."""


class QueryAggregationRangeType(core.ModelBase):
    """QueryAggregationRangeType"""

    sub_type: QueryAggregationRangeSubType = pydantic.Field(alias=str("subType"))  # type: ignore[literal-required]
    type: typing.Literal["range"] = "range"


QueryAggregationValueType: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[core_models.DateType, core_models.DoubleType, core_models.TimestampType],
    pydantic.Field(discriminator="type"),
]
"""A union of all the types supported by query aggregation keys."""


QueryApiName: typing_extensions.TypeAlias = str
"""The name of the Query in the API."""


class QueryArrayType(core.ModelBase):
    """QueryArrayType"""

    sub_type: QueryDataType = pydantic.Field(alias=str("subType"))  # type: ignore[literal-required]
    type: typing.Literal["array"] = "array"


QueryDataType: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        core_models.DateType,
        "OntologyInterfaceObjectType",
        "QueryStructType",
        core_models.StringType,
        core_models.IntegerType,
        "ThreeDimensionalAggregation",
        core_models.FloatType,
        core_models.LongType,
        core_models.UnsupportedType,
        core_models.AttachmentType,
        "QueryArrayType",
        "OntologyObjectSetType",
        "TwoDimensionalAggregation",
        "QueryTypeReferenceType",
        core_models.TimestampType,
        "QuerySetType",
        core_models.VoidType,
        "EntrySetType",
        core_models.DoubleType,
        "QueryUnionType",
        core_models.BooleanType,
        core_models.MediaReferenceType,
        core_models.NullType,
        "OntologyInterfaceObjectSetType",
        "OntologyObjectType",
    ],
    pydantic.Field(discriminator="type"),
]
"""A union of all the types supported by Ontology Query parameters or outputs."""


class QueryParameterV2(core.ModelBase):
    """Details about a parameter of a query."""

    description: typing.Optional[str] = None
    data_type: QueryDataType = pydantic.Field(alias=str("dataType"))  # type: ignore[literal-required]
    required: bool


QueryRuntimeErrorParameter: typing_extensions.TypeAlias = str
"""QueryRuntimeErrorParameter"""


class QuerySetType(core.ModelBase):
    """QuerySetType"""

    sub_type: QueryDataType = pydantic.Field(alias=str("subType"))  # type: ignore[literal-required]
    type: typing.Literal["set"] = "set"


class QueryStructField(core.ModelBase):
    """QueryStructField"""

    name: core_models.StructFieldName
    field_type: QueryDataType = pydantic.Field(alias=str("fieldType"))  # type: ignore[literal-required]


class QueryStructType(core.ModelBase):
    """QueryStructType"""

    fields: typing.List[QueryStructField]
    type: typing.Literal["struct"] = "struct"


class QueryThreeDimensionalAggregation(core.ModelBase):
    """QueryThreeDimensionalAggregation"""

    groups: typing.List[NestedQueryAggregation]


class QueryTwoDimensionalAggregation(core.ModelBase):
    """QueryTwoDimensionalAggregation"""

    groups: typing.List[QueryAggregation]


class QueryTypeReferenceType(core.ModelBase):
    """
    A reference to a type that is defined in the `typeReferences` map of the enclosing Query.
    This enables support for recursive type definitions where a type may reference itself.
    """

    type_id: TypeReferenceIdentifier = pydantic.Field(alias=str("typeId"))  # type: ignore[literal-required]
    type: typing.Literal["typeReference"] = "typeReference"


class QueryTypeV2(core.ModelBase):
    """Represents a query type in the Ontology."""

    api_name: QueryApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    description: typing.Optional[str] = None
    display_name: typing.Optional[core_models.DisplayName] = pydantic.Field(alias=str("displayName"), default=None)  # type: ignore[literal-required]
    parameters: typing.Dict[ParameterId, QueryParameterV2]
    output: QueryDataType
    rid: FunctionRid
    version: FunctionVersion
    type_references: typing.Dict[TypeReferenceIdentifier, QueryDataType] = pydantic.Field(alias=str("typeReferences"))  # type: ignore[literal-required]


class QueryUnionType(core.ModelBase):
    """QueryUnionType"""

    union_types: typing.List[QueryDataType] = pydantic.Field(alias=str("unionTypes"))  # type: ignore[literal-required]
    type: typing.Literal["union"] = "union"


class RangeAllowedValues(core.ModelBase):
    """The parameter value must fall within the specified numeric range."""

    gt: typing.Optional[ParameterConstraintValue] = None
    gte: typing.Optional[ParameterConstraintValue] = None
    lt: typing.Optional[ParameterConstraintValue] = None
    lte: typing.Optional[ParameterConstraintValue] = None
    type: typing.Literal["range"] = "range"


class RangeConstraint(core.ModelBase):
    """The parameter value must be within the defined range."""

    lt: typing.Optional[typing.Any] = None
    """Less than"""

    lte: typing.Optional[typing.Any] = None
    """Less than or equal"""

    gt: typing.Optional[typing.Any] = None
    """Greater than"""

    gte: typing.Optional[typing.Any] = None
    """Greater than or equal"""

    type: typing.Literal["range"] = "range"


class RangesConstraint(core.ModelBase):
    """RangesConstraint"""

    minimum_value: typing.Optional[PropertyValue] = pydantic.Field(alias=str("minimumValue"), default=None)  # type: ignore[literal-required]
    maximum_value: typing.Optional[PropertyValue] = pydantic.Field(alias=str("maximumValue"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["range"] = "range"


class Reason(core.ModelBase):
    """Reason"""

    reason: ReasonType
    type: typing.Literal["reason"] = "reason"


ReasonType: typing_extensions.TypeAlias = typing.Literal["USER_CLOSED", "CHANNEL_CLOSED"]
"""Represents the reason a subscription was closed."""


class ReferenceSigningOptions(core.ModelBase):
    """Options for signing references in the response."""

    sign_media_references: typing.Optional[bool] = pydantic.Field(alias=str("signMediaReferences"), default=None)  # type: ignore[literal-required]
    """
    Deprecated and ignored. Media references backed by a media set view are signed by default:
    the response includes a `token` on each `MediaReference` value that can be used to access the
    referenced media item directly, enabling item-level access control (the caller needs access to
    the object whose property holds the reference, not view access on the parent media set).

    This field no longer has any effect; setting it to true or false is ignored. Arrays of media
    references are not signed.
    """


class ReferenceUpdate(core.ModelBase):
    """
    The updated data value associated with an object instance's external reference. The object instance
    is uniquely identified by an object type and a primary key. Note that the value of the property
    field returns a dereferenced value rather than the reference itself.
    """

    object_type: ObjectTypeApiName = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]
    primary_key: ObjectPrimaryKey = pydantic.Field(alias=str("primaryKey"))  # type: ignore[literal-required]
    property: PropertyApiName
    value: ReferenceValue
    type: typing.Literal["reference"] = "reference"


class RefreshObjectSet(core.ModelBase):
    """The list of updated Foundry Objects cannot be provided. The object set must be refreshed using Object Set Service."""

    id: SubscriptionId
    object_type: ObjectTypeApiName = pydantic.Field(alias=str("objectType"))  # type: ignore[literal-required]
    type: typing.Literal["refreshObjectSet"] = "refreshObjectSet"


class RegexConstraint(core.ModelBase):
    """RegexConstraint"""

    pattern: str
    partial_match: bool = pydantic.Field(alias=str("partialMatch"))  # type: ignore[literal-required]
    type: typing.Literal["regex"] = "regex"


class RegexQuery(core.ModelBase):
    """
    Returns objects where the specified field matches the regex pattern provided. This applies to the non-analyzed
    form of text fields. Supported operators:
      - `.` matches any character.
      - `?` repeats the previous character 0 or 1 times.
      - `+` repeats the previous character 1 or more times.
      - `*` repeats the previous character 0 or more times.
      - `{}` defines the minimum and maximum number of times the preceding character can repeat. `{2}` means the
        previous character must repeat only twice, `{2,}` means the previous character must repeat at least twice,
        and `{2,4}` means the previous character must repeat between 2-4 times.
      - `|` is the OR operator.
      - `()` forms a group within an expression such that the group can be treated as a single character.
      - `[]` matches a single one of the characters contained inside the brackets, meaning [abc] matches `a`, `b` or
        `c`. Unless `-` is the first character or escaped with `\\` (in which case it is treated as a normal character),
        `-` can be used inside the bracket to create a range of characters, meaning [a-c] matches `a`, `b`, or `c`.
        If the character sequence inside the brackets begins with `^`, the set of characters is negated, meaning
        [^abc] does not match `a`, `b`, or `c`. Otherwise, `^` is treated as a normal character.
      - `"` creates groups of string literals.
      - `\\` is used as an escape character. However, \\d and \\D match digit and non-digit characters respectively, \\s
      and \\S match whitespace and non whitespace characters respectively, and \\w and \\W match word and non word
      characters respectively.

    Either `field` or `propertyIdentifier` can be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: str
    type: typing.Literal["regex"] = "regex"


class RelativeDateRangeQuery(core.ModelBase):
    """
    Returns objects where the specified date or timestamp property falls within a relative date range.
    The bounds are calculated relative to query execution time and rounded to midnight in the specified timezone.
    """

    field: typing.Optional[PropertyApiName] = None
    """The property API name to filter on (either field or propertyIdentifier must be provided)."""

    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    """The property identifier to filter on (either field or propertyIdentifier must be provided)."""

    relative_start_time: typing.Optional[RelativeDateRangeBound] = pydantic.Field(alias=str("relativeStartTime"), default=None)  # type: ignore[literal-required]
    """
    The lower bound relative to query time (inclusive). Negative values go into the past.
    For example, { value: -7, timeUnit: DAY } means 7 days ago.
    """

    relative_end_time: typing.Optional[RelativeDateRangeBound] = pydantic.Field(alias=str("relativeEndTime"), default=None)  # type: ignore[literal-required]
    """
    The upper bound relative to query time (exclusive). Negative values go into the past.
    For example, { value: 1, timeUnit: MONTH } means the start of next month.
    """

    time_zone_id: str = pydantic.Field(alias=str("timeZoneId"))  # type: ignore[literal-required]
    """
    Time zone ID for midnight calculation (e.g., "America/New_York", "Europe/London", "Etc/UTC").
    See https://en.wikipedia.org/wiki/List_of_tz_database_time_zones for valid values.
    """

    type: typing.Literal["relativeDateRange"] = "relativeDateRange"


RelativeDatetimeDuration: typing_extensions.TypeAlias = core.Long
"""The magnitude of a relative datetime offset."""


RelativeDatetimeTense: typing_extensions.TypeAlias = typing.Literal["FUTURE", "PAST"]
"""Direction of a relative datetime offset."""


RelativeDatetimeUnit: typing_extensions.TypeAlias = typing.Literal[
    "SECOND", "MINUTE", "HOUR", "DAY", "WEEK"
]
"""Time unit for relative datetime offsets."""


class RelativeDatetimeValue(core.ModelBase):
    """A datetime expressed as an offset from the current time."""

    duration: RelativeDatetimeDuration
    unit: RelativeDatetimeUnit
    tense: RelativeDatetimeTense
    type: typing.Literal["relative"] = "relative"


class RelativePointInTime(core.ModelBase):
    """A point in time specified relative to query execution time."""

    value: int
    """The numeric value of the time offset. Negative values indicate the past, positive values the future."""

    time_unit: RelativeTimeUnit = pydantic.Field(alias=str("timeUnit"))  # type: ignore[literal-required]
    """The unit of time for the value."""

    type: typing.Literal["relativePoint"] = "relativePoint"


class RelativeTime(core.ModelBase):
    """A relative time, such as "3 days before" or "2 hours after" the current moment."""

    when: RelativeTimeRelation
    value: int
    unit: RelativeTimeSeriesTimeUnit


class RelativeTimeRange(core.ModelBase):
    """A relative time range for a time series query."""

    start_time: typing.Optional[RelativeTime] = pydantic.Field(alias=str("startTime"), default=None)  # type: ignore[literal-required]
    end_time: typing.Optional[RelativeTime] = pydantic.Field(alias=str("endTime"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["relative"] = "relative"


RelativeTimeRelation: typing_extensions.TypeAlias = typing.Literal["BEFORE", "AFTER"]
"""RelativeTimeRelation"""


RelativeTimeSeriesTimeUnit: typing_extensions.TypeAlias = typing.Literal[
    "MILLISECONDS", "SECONDS", "MINUTES", "HOURS", "DAYS", "WEEKS", "MONTHS", "YEARS"
]
"""RelativeTimeSeriesTimeUnit"""


RelativeTimeUnit: typing_extensions.TypeAlias = typing.Literal["DAY", "WEEK", "MONTH", "YEAR"]
"""Units for relative time calculations."""


RequestId: typing_extensions.TypeAlias = core.UUID
"""Unique request id"""


class ResolvedInterfacePropertyType(core.ModelBase):
    """
    An interface property type with additional fields to indicate constraints that need to be satisfied by
    implementing object property types.
    """

    rid: InterfacePropertyTypeRid
    api_name: InterfacePropertyApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    display_name: core_models.DisplayName = pydantic.Field(alias=str("displayName"))  # type: ignore[literal-required]
    description: typing.Optional[str] = None
    """A short text that describes the InterfacePropertyType."""

    data_type: ObjectPropertyType = pydantic.Field(alias=str("dataType"))  # type: ignore[literal-required]
    value_type_api_name: typing.Optional[ValueTypeApiName] = pydantic.Field(alias=str("valueTypeApiName"), default=None)  # type: ignore[literal-required]
    value_formatting: typing.Optional[PropertyValueFormattingRule] = pydantic.Field(alias=str("valueFormatting"), default=None)  # type: ignore[literal-required]
    require_implementation: bool = pydantic.Field(alias=str("requireImplementation"))  # type: ignore[literal-required]
    """Whether each implementing object type must declare an implementation for this property."""


RestrictedViewRid: typing_extensions.TypeAlias = core.RID
"""The RID of a Foundry restricted view."""


ReturnEditsMode: typing_extensions.TypeAlias = typing.Literal[
    "ALL", "ALL_V2_WITH_DELETIONS", "NONE"
]
"""If not specified, defaults to `NONE`."""


class RevertActionEnabledActionTypesQueryV2(core.ModelBase):
    """Returns action types based on whether they are revertible."""

    value: bool
    type: typing.Literal["revertActionEnabled"] = "revertActionEnabled"


class RidConstraint(core.ModelBase):
    """The string must be a valid RID (Resource Identifier)."""

    type: typing.Literal["rid"] = "rid"


class RollingAggregateWindowPoints(core.ModelBase):
    """Number of points in each window."""

    count: int
    type: typing.Literal["pointsCount"] = "pointsCount"


SdkPackageName: typing_extensions.TypeAlias = str
"""SdkPackageName"""


SdkPackageRid: typing_extensions.TypeAlias = core.RID
"""SdkPackageRid"""


SdkVersion: typing_extensions.TypeAlias = str
"""SdkVersion"""


class SearchActionTypesFullMetadataRequest(core.ModelBase):
    """SearchActionTypesFullMetadataRequest"""

    where: typing.Optional[ActionTypeSearchJsonQueryV2] = None
    order_by: typing.Optional[SearchActionTypesOrderByV2] = pydantic.Field(alias=str("orderBy"), default=None)  # type: ignore[literal-required]
    fuzziness: typing.Optional[ActionTypeFuzziness] = None
    page_size: typing.Optional[core_models.PageSize] = pydantic.Field(alias=str("pageSize"), default=None)  # type: ignore[literal-required]
    page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("pageToken"), default=None)  # type: ignore[literal-required]


class SearchActionTypesFullMetadataResponse(core.ModelBase):
    """SearchActionTypesFullMetadataResponse"""

    data: typing.List[ActionTypeFullMetadata]
    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    total_count: core_models.TotalCount = pydantic.Field(alias=str("totalCount"))  # type: ignore[literal-required]


class SearchActionTypesOrderByV2(core.ModelBase):
    """
    Specifies the ordering of action type search results by a field and an ordering direction. If not provided,
    results are ordered by relevance of the match.
    """

    field: ActionTypeSortByV2
    direction: typing.Optional[str] = None
    """Specifies the ordering direction (can be either `asc` or `desc`)"""


class SearchActionTypesRequestV2(core.ModelBase):
    """SearchActionTypesRequestV2"""

    where: typing.Optional[ActionTypeSearchJsonQueryV2] = None
    order_by: typing.Optional[SearchActionTypesOrderByV2] = pydantic.Field(alias=str("orderBy"), default=None)  # type: ignore[literal-required]
    fuzziness: typing.Optional[ActionTypeFuzziness] = None
    page_size: typing.Optional[core_models.PageSize] = pydantic.Field(alias=str("pageSize"), default=None)  # type: ignore[literal-required]
    page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("pageToken"), default=None)  # type: ignore[literal-required]


class SearchActionTypesResponseV2(core.ModelBase):
    """SearchActionTypesResponseV2"""

    data: typing.List[ActionTypeV2]
    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    total_count: core_models.TotalCount = pydantic.Field(alias=str("totalCount"))  # type: ignore[literal-required]


SearchJsonQueryV2: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "LtQueryV2",
        "DoesNotIntersectBoundingBoxQuery",
        "RelativeDateRangeQuery",
        "WildcardQuery",
        "WithinDistanceOfQuery",
        "WithinBoundingBoxQuery",
        "NotQueryV2",
        "IntersectsBoundingBoxQuery",
        "AndQueryV2",
        "ContainsAllTermsInOrderPrefixLastTerm",
        "GteQueryV2",
        "ContainsAllTermsInOrderQuery",
        "WithinPolygonQuery",
        "IntersectsPolygonQuery",
        "LteQueryV2",
        "OrQueryV2",
        "InQuery",
        "DoesNotIntersectPolygonQuery",
        "EqualsQueryV2",
        "ContainsAllTermsQuery",
        "GtQueryV2",
        "ContainsQueryV2",
        "RegexQuery",
        "IsNullQueryV2",
        "ContainsAnyTermQuery",
        "IntervalQuery",
        "GeoShapeV2Query",
        "StartsWithQuery",
    ],
    pydantic.Field(discriminator="type"),
]
"""SearchJsonQueryV2"""


class SearchObjectsForInterfaceRequest(core.ModelBase):
    """SearchObjectsForInterfaceRequest"""

    where: typing.Optional[SearchJsonQueryV2] = None
    order_by: typing.Optional[SearchOrderByV2] = pydantic.Field(alias=str("orderBy"), default=None)  # type: ignore[literal-required]
    augmented_properties: typing.Dict[ObjectTypeApiName, typing.List[PropertyApiName]] = pydantic.Field(alias=str("augmentedProperties"))  # type: ignore[literal-required]
    """
    A map from object type API name to a list of property type API names. For each returned object, if the 
    object’s object type is a key in the map, then we augment the response for that object type with the list 
    of properties specified in the value.
    """

    augmented_shared_property_types: typing.Dict[InterfaceTypeApiName, typing.List[SharedPropertyTypeApiName]] = pydantic.Field(alias=str("augmentedSharedPropertyTypes"))  # type: ignore[literal-required]
    """
    A map from interface type API name to a list of shared property type API names. For each returned object, if
    the object implements an interface that is a key in the map, then we augment the response for that object 
    type with the list of properties specified in the value.
    """

    augmented_interface_property_types: typing.Dict[InterfaceTypeApiName, typing.List[InterfacePropertyApiName]] = pydantic.Field(alias=str("augmentedInterfacePropertyTypes"))  # type: ignore[literal-required]
    """
    A map from interface type API name to a list of interface property type API names. For each returned object, 
    if the object implements an interface that is a key in the map, then we augment the response for that object 
    type with the list of properties specified in the value.
    """

    selected_shared_property_types: typing.List[SharedPropertyTypeApiName] = pydantic.Field(alias=str("selectedSharedPropertyTypes"))  # type: ignore[literal-required]
    """
    A list of shared property type API names of the interface type that should be included in the response. 
    Omit this parameter to include all properties of the interface type in the response.
    """

    selected_interface_property_types: typing.List[InterfacePropertyApiName] = pydantic.Field(alias=str("selectedInterfacePropertyTypes"))  # type: ignore[literal-required]
    """
    A list of interface property type API names of the interface type that should be included in the response. 
    Omit this parameter to include all properties of the interface type in the response.
    """

    selected_object_types: typing.List[ObjectTypeApiName] = pydantic.Field(alias=str("selectedObjectTypes"))  # type: ignore[literal-required]
    """
    A list of object type API names that should be included in the response. If non-empty, object types that are
    not mentioned will not be included in the response even if they implement the specified interface. Omit the 
    parameter to include all object types.
    """

    other_interface_types: typing.List[InterfaceTypeApiName] = pydantic.Field(alias=str("otherInterfaceTypes"))  # type: ignore[literal-required]
    """
    A list of interface type API names. Object types must implement all the mentioned interfaces in order to be 
    included in the response.
    """

    page_size: typing.Optional[core_models.PageSize] = pydantic.Field(alias=str("pageSize"), default=None)  # type: ignore[literal-required]
    page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("pageToken"), default=None)  # type: ignore[literal-required]


class SearchObjectsRequestV2(core.ModelBase):
    """SearchObjectsRequestV2"""

    where: typing.Optional[SearchJsonQueryV2] = None
    order_by: typing.Optional[SearchOrderByV2] = pydantic.Field(alias=str("orderBy"), default=None)  # type: ignore[literal-required]
    page_size: typing.Optional[core_models.PageSize] = pydantic.Field(alias=str("pageSize"), default=None)  # type: ignore[literal-required]
    page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("pageToken"), default=None)  # type: ignore[literal-required]
    select: typing.List[PropertyApiName]
    """The API names of the object type properties to include in the response."""

    select_v2: typing.Optional[typing.List[PropertyIdentifier]] = pydantic.Field(alias=str("selectV2"), default=None)  # type: ignore[literal-required]
    """
    The identifiers of the properties to include in the response. Only selectV2 or select should be populated,
    but not both.
    """

    default_load_level: typing.Optional[PropertyLoadLevel] = pydantic.Field(alias=str("defaultLoadLevel"), default=None)  # type: ignore[literal-required]
    exclude_rid: typing.Optional[bool] = pydantic.Field(alias=str("excludeRid"), default=None)  # type: ignore[literal-required]
    """
    A flag to exclude the retrieval of the `__rid` property.
    Setting this to true may improve performance of this endpoint for object types in OSV2.
    """

    snapshot: typing.Optional[bool] = None
    """
    A flag to use snapshot consistency when paging.
    Setting this to true will give you a consistent view from before you start paging through the results, ensuring you do not get duplicate or missing items.
    Setting this to false will let new results enter as you page, but you may encounter duplicate or missing items.
    This defaults to false if not specified, which means you will always get the latest results.
    """

    reference_signing_options: typing.Optional[ReferenceSigningOptions] = pydantic.Field(alias=str("referenceSigningOptions"), default=None)  # type: ignore[literal-required]


class SearchObjectsResponseV2(core.ModelBase):
    """SearchObjectsResponseV2"""

    data: typing.List[OntologyObjectV2]
    next_page_token: typing.Optional[core_models.PageToken] = pydantic.Field(alias=str("nextPageToken"), default=None)  # type: ignore[literal-required]
    total_count: core_models.TotalCount = pydantic.Field(alias=str("totalCount"))  # type: ignore[literal-required]


SearchOrderByType: typing_extensions.TypeAlias = typing.Literal["fields", "relevance"]
"""SearchOrderByType"""


class SearchOrderByV2(core.ModelBase):
    """
    Specifies the ordering of search results by a field and an ordering direction, or by relevance.
    If the `fields` array is provided, `orderType` is automatically set to `fields`.
    If this object is omitted entirely, the ordering is unspecified.

    Setting `orderType` to `relevance` requests that results are sorted by decreasing relevance score.
    For queries that include text search filters (e.g. `containsAllTerms`, `containsAnyTerm`,
    `containsAllTermsInOrder`, `containsAllTermsInOrderPrefixLastTerm`) or `nearestNeighbors`, the
    relevance score reflects how well each object matches the query. For other queries, the ordering
    is unspecified.

    When paging through results ordered by relevance, ordering is not guaranteed to be consistent
    across pages: an object may appear on multiple pages or be skipped entirely. Use a single page
    when result completeness is required.

    Relevance ordering can be expensive and should only be used when required.
    """

    order_type: typing.Optional[SearchOrderByType] = pydantic.Field(alias=str("orderType"), default=None)  # type: ignore[literal-required]
    fields: typing.List[SearchOrderingV2]


class SearchOrderingV2(core.ModelBase):
    """SearchOrderingV2"""

    field: PropertyApiName
    direction: typing.Optional[str] = None
    """Specifies the ordering direction (can be either `asc` or `desc`)"""


class SectionRidActionTypesQueryV2(core.ModelBase):
    """Returns action types with a section matching the given section rid."""

    value: ActionSectionRid
    type: typing.Literal["sectionRid"] = "sectionRid"


SelectedPropertyApiName: typing_extensions.TypeAlias = str
"""
By default, whenever an object is requested, all of its properties are returned, except for properties of the 
following types:
- Vector

The response can be filtered to only include certain properties using the `properties` query parameter. Note
that ontology object set endpoints refer to this parameter as `select`.

Properties to include can be specified in one of two ways.

- A comma delimited list as the value for the `properties` query parameter
  `properties={property1ApiName},{property2ApiName}`
- Multiple `properties` query parameters.
  `properties={property1ApiName}&properties={property2ApiName}`

The primary key of the object will always be returned even if it wasn't specified in the `properties` values.

Unknown properties specified in the `properties` list will result in a `PropertiesNotFound` error.

To find the API name for your property, use the `Get object type` endpoint or check the **Ontology Manager**.
"""


class SelectedPropertyApproximateDistinctAggregation(core.ModelBase):
    """Computes an approximate number of distinct values for the provided field."""

    selected_property_api_name: PropertyApiName = pydantic.Field(alias=str("selectedPropertyApiName"))  # type: ignore[literal-required]
    type: typing.Literal["approximateDistinct"] = "approximateDistinct"


class SelectedPropertyApproximatePercentileAggregation(core.ModelBase):
    """Computes the approximate percentile value for the provided field."""

    selected_property_api_name: PropertyApiName = pydantic.Field(alias=str("selectedPropertyApiName"))  # type: ignore[literal-required]
    approximate_percentile: float = pydantic.Field(alias=str("approximatePercentile"))  # type: ignore[literal-required]
    type: typing.Literal["approximatePercentile"] = "approximatePercentile"


class SelectedPropertyAvgAggregation(core.ModelBase):
    """Computes the average value for the provided field."""

    selected_property_api_name: PropertyApiName = pydantic.Field(alias=str("selectedPropertyApiName"))  # type: ignore[literal-required]
    type: typing.Literal["avg"] = "avg"


class SelectedPropertyCollectListAggregation(core.ModelBase):
    """
    Lists all values of a property up to the specified limit. The maximum supported limit is 100, by default.

    NOTE: A separate count aggregation should be used to determine the total count of values, to account for
    a possible truncation of the returned list.

    Ignores objects for which a property is absent, so the returned list will contain non-null values only.
    Returns an empty list when none of the objects have values for a provided property.
    """

    selected_property_api_name: PropertyApiName = pydantic.Field(alias=str("selectedPropertyApiName"))  # type: ignore[literal-required]
    limit: int
    """Maximum number of values to collect. The maximum supported limit is 100."""

    type: typing.Literal["collectList"] = "collectList"


class SelectedPropertyCollectSetAggregation(core.ModelBase):
    """
    Lists all distinct values of a property up to the specified limit. The maximum supported limit is 100.

    NOTE: A separate cardinality / exactCardinality aggregation should be used to determine the total count of
    values, to account for a possible truncation of the returned set.

    Ignores objects for which a property is absent, so the returned list will contain non-null values only.
    Returns an empty list when none of the objects have values for a provided property.
    """

    selected_property_api_name: PropertyApiName = pydantic.Field(alias=str("selectedPropertyApiName"))  # type: ignore[literal-required]
    limit: int
    """Maximum number of values to collect. The maximum supported limit is 100."""

    type: typing.Literal["collectSet"] = "collectSet"


class SelectedPropertyCountAggregation(core.ModelBase):
    """Computes the total count of objects."""

    type: typing.Literal["count"] = "count"


class SelectedPropertyExactDistinctAggregation(core.ModelBase):
    """Computes an exact number of distinct values for the provided field. May be slower than an approximate distinct aggregation."""

    selected_property_api_name: PropertyApiName = pydantic.Field(alias=str("selectedPropertyApiName"))  # type: ignore[literal-required]
    type: typing.Literal["exactDistinct"] = "exactDistinct"


class SelectedPropertyExpression(core.ModelBase):
    """Definition for a selected property over a MethodObjectSet."""

    object_set: MethodObjectSet = pydantic.Field(alias=str("objectSet"))  # type: ignore[literal-required]
    operation: SelectedPropertyOperation
    type: typing.Literal["selection"] = "selection"


class SelectedPropertyMaxAggregation(core.ModelBase):
    """Computes the maximum value for the provided field."""

    selected_property_api_name: PropertyApiName = pydantic.Field(alias=str("selectedPropertyApiName"))  # type: ignore[literal-required]
    type: typing.Literal["max"] = "max"


class SelectedPropertyMinAggregation(core.ModelBase):
    """Computes the minimum value for the provided field."""

    selected_property_api_name: PropertyApiName = pydantic.Field(alias=str("selectedPropertyApiName"))  # type: ignore[literal-required]
    type: typing.Literal["min"] = "min"


SelectedPropertyOperation: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "SelectedPropertyApproximateDistinctAggregation",
        "SelectedPropertyMinAggregation",
        "SelectedPropertyAvgAggregation",
        "SelectedPropertyMaxAggregation",
        "SelectedPropertyApproximatePercentileAggregation",
        "GetSelectedPropertyOperation",
        "SelectedPropertyCountAggregation",
        "SelectedPropertySumAggregation",
        "SelectedPropertyCollectListAggregation",
        "SelectedPropertyExactDistinctAggregation",
        "SelectedPropertyCollectSetAggregation",
    ],
    pydantic.Field(discriminator="type"),
]
"""Operation on a selected property, can be an aggregation function or retrieval of a single selected property"""


class SelectedPropertySumAggregation(core.ModelBase):
    """Computes the sum of values for the provided field."""

    selected_property_api_name: PropertyApiName = pydantic.Field(alias=str("selectedPropertyApiName"))  # type: ignore[literal-required]
    type: typing.Literal["sum"] = "sum"


class SharedPropertyType(core.ModelBase):
    """A property type that can be shared across object types."""

    rid: SharedPropertyTypeRid
    api_name: SharedPropertyTypeApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    display_name: core_models.DisplayName = pydantic.Field(alias=str("displayName"))  # type: ignore[literal-required]
    description: typing.Optional[str] = None
    """A short text that describes the SharedPropertyType."""

    data_type: ObjectPropertyType = pydantic.Field(alias=str("dataType"))  # type: ignore[literal-required]
    value_type_api_name: typing.Optional[ValueTypeApiName] = pydantic.Field(alias=str("valueTypeApiName"), default=None)  # type: ignore[literal-required]
    value_formatting: typing.Optional[PropertyValueFormattingRule] = pydantic.Field(alias=str("valueFormatting"), default=None)  # type: ignore[literal-required]
    type_classes: typing.Optional[typing.List[TypeClass]] = pydantic.Field(alias=str("typeClasses"), default=None)  # type: ignore[literal-required]


SharedPropertyTypeApiName: typing_extensions.TypeAlias = str
"""
The name of the shared property type in the API in lowerCamelCase format. To find the API name for your
shared property type, use the `List shared property types` endpoint or check the **Ontology Manager**.
"""


SharedPropertyTypeRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of an shared property type, useful for interacting with other Foundry APIs."""


SpatialFilterMode: typing_extensions.TypeAlias = typing.Literal[
    "INTERSECTS", "DISJOINT", "WITHIN", "CONTAINS"
]
"""
The spatial relation operator for a GeoShapeV2Query. INTERSECTS matches objects that intersect the provided
geometry, DISJOINT matches objects that do not intersect the provided geometry, WITHIN matches objects that
lie within the provided geometry, and CONTAINS matches objects that contain the provided geometry.
"""


class StartsWithQuery(core.ModelBase):
    """
    Deprecated alias for `containsAllTermsInOrderPrefixLastTerm`, which is preferred because the name `startsWith` is misleading.
    Returns objects where the specified field starts with the provided value. Allows you to specify a property to
    query on by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: str
    type: typing.Literal["startsWith"] = "startsWith"


class StaticArgument(core.ModelBase):
    """Represents a static argument in a logic rule."""

    value: DataValue
    type: typing.Literal["staticValue"] = "staticValue"


class StaticConstraintValue(core.ModelBase):
    """A literal constraint value."""

    value: DataValue
    type: typing.Literal["static"] = "static"


class StatusActionTypesQueryV2(core.ModelBase):
    """Returns action types with the given status."""

    value: ActionTypeStatusFilter
    type: typing.Literal["status"] = "status"


StreamMessage: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "ObjectSetUpdates", "RefreshObjectSet", "SubscriptionClosed", "ObjectSetSubscribeResponses"
    ],
    pydantic.Field(discriminator="type"),
]
"""StreamMessage"""


StreamRid: typing_extensions.TypeAlias = core.RID
"""The RID of a Foundry stream."""


class StreamTimeSeriesPointsRequest(core.ModelBase):
    """StreamTimeSeriesPointsRequest"""

    range: typing.Optional[TimeRange] = None
    aggregate: typing.Optional[AggregateTimeSeries] = None


class StreamTimeSeriesValuesRequest(core.ModelBase):
    """StreamTimeSeriesValuesRequest"""

    range: typing.Optional[TimeRange] = None


StreamingOutputFormat: typing_extensions.TypeAlias = typing.Literal["JSON", "ARROW"]
"""
Which format to serialize the binary stream in.
ARROW is more efficient for streaming a large sized response.
"""


class StringConstant(core.ModelBase):
    """StringConstant"""

    value: str
    type: typing.Literal["constant"] = "constant"


class StringLengthConstraint(core.ModelBase):
    """
    The parameter value must have a length within the defined range.
    *This range is always inclusive.*
    """

    lt: typing.Optional[typing.Any] = None
    """Less than"""

    lte: typing.Optional[typing.Any] = None
    """Less than or equal"""

    gt: typing.Optional[typing.Any] = None
    """Greater than"""

    gte: typing.Optional[typing.Any] = None
    """Greater than or equal"""

    type: typing.Literal["stringLength"] = "stringLength"


class StringRegexMatchConstraint(core.ModelBase):
    """The parameter value must match a predefined regular expression."""

    regex: str
    """The regular expression configured in the **Ontology Manager**."""

    configured_failure_message: typing.Optional[str] = pydantic.Field(alias=str("configuredFailureMessage"), default=None)  # type: ignore[literal-required]
    """
    The message indicating that the regular expression was not matched.
    This is configured per parameter in the **Ontology Manager**.
    """

    type: typing.Literal["stringRegexMatch"] = "stringRegexMatch"


class StringValue(core.ModelBase):
    """StringValue"""

    value: str
    type: typing.Literal["stringValue"] = "stringValue"


class StructConstraint(core.ModelBase):
    """StructConstraint"""

    properties: typing.Dict[PropertyApiName, ValueTypeApiName]
    """A map of the properties of the struct type to the value type applied to that property."""

    type: typing.Literal["struct"] = "struct"


class StructEvaluatedConstraint(core.ModelBase):
    """Represents the validity of a singleton struct parameter."""

    struct_fields: typing.Dict[StructParameterFieldApiName, StructFieldEvaluationResult] = pydantic.Field(alias=str("structFields"))  # type: ignore[literal-required]
    type: typing.Literal["struct"] = "struct"


StructFieldApiName: typing_extensions.TypeAlias = str
"""The name of a struct field in the Ontology."""


StructFieldArgument: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["StructListParameterFieldArgument", "StructParameterFieldArgument"],
    pydantic.Field(discriminator="type"),
]
"""Represents an argument used for an individual struct field."""


StructFieldEvaluatedConstraint: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "OneOfConstraint",
        "RangeConstraint",
        "ObjectQueryResultConstraint",
        "StringLengthConstraint",
        "StringRegexMatchConstraint",
    ],
    pydantic.Field(discriminator="type"),
]
"""
A constraint that an action struct parameter field value must satisfy in order to be considered valid.
Constraints can be configured on fields of struct parameters in the **Ontology Manager**. 
Applicable constraints are determined dynamically based on parameter inputs. 
Parameter values are evaluated against the final set of constraints.

The type of the constraint.
| Type                  | Description                                                                                                                                                                                                                     |
|-----------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `oneOf`               | The struct parameter field has a manually predefined set of options.                                                                                                                                                            |
| `range`               | The struct parameter field value must be within the defined range.                                                                                                                                                              |
| `stringLength`        | The struct parameter field value must have a length within the defined range.                                                                                                                                                   |
| `stringRegexMatch`    | The struct parameter field value must match a predefined regular expression.                                                                                                                                                    |
| `objectQueryResult`   | The struct parameter field value must be the primary key of an object found within an object set.                                                                                                                               |
"""


class StructFieldEvaluationResult(core.ModelBase):
    """Represents the validity of a struct parameter's fields against the configured constraints."""

    result: ValidationResult
    evaluated_constraints: typing.List[StructFieldEvaluatedConstraint] = pydantic.Field(alias=str("evaluatedConstraints"))  # type: ignore[literal-required]
    required: bool
    """Represents whether the parameter is a required input to the action."""


class StructFieldOfPropertyImplementation(core.ModelBase):
    """StructFieldOfPropertyImplementation"""

    property_api_name: PropertyApiName = pydantic.Field(alias=str("propertyApiName"))  # type: ignore[literal-required]
    struct_field_api_name: StructFieldApiName = pydantic.Field(alias=str("structFieldApiName"))  # type: ignore[literal-required]
    type: typing.Literal["structFieldOfProperty"] = "structFieldOfProperty"


class StructFieldPropertyMapping(core.ModelBase):
    """A single struct field's mapping where `apiName` is the name of a struct field."""

    api_name: StructFieldApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]


class StructFieldSelector(core.ModelBase):
    """
    A combination of a property identifier and the load level to apply to the property. You can select a reduced
    value for arrays and the main value for structs. If the provided load level cannot be applied to the property
    type, then it will be ignored. This selector is experimental and may not work in filters or sorts.
    """

    property_api_name: PropertyApiName = pydantic.Field(alias=str("propertyApiName"))  # type: ignore[literal-required]
    struct_field_api_name: StructFieldApiName = pydantic.Field(alias=str("structFieldApiName"))  # type: ignore[literal-required]
    type: typing.Literal["structField"] = "structField"


class StructFieldType(core.ModelBase):
    """StructFieldType"""

    api_name: StructFieldApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    rid: StructFieldTypeRid
    data_type: ObjectPropertyType = pydantic.Field(alias=str("dataType"))  # type: ignore[literal-required]
    type_classes: typing.Optional[typing.List[TypeClass]] = pydantic.Field(alias=str("typeClasses"), default=None)  # type: ignore[literal-required]


StructFieldTypeRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of a struct field, useful for interacting with other Foundry APIs."""


class StructListParameterFieldArgument(core.ModelBase):
    """Represents a struct list parameter field argument in a logic rule."""

    parameter_id: ParameterId = pydantic.Field(alias=str("parameterId"))  # type: ignore[literal-required]
    struct_parameter_field_api_name: StructParameterFieldApiName = pydantic.Field(alias=str("structParameterFieldApiName"))  # type: ignore[literal-required]
    type: typing.Literal["structListParameterFieldValue"] = "structListParameterFieldValue"


StructParameterFieldApiName: typing_extensions.TypeAlias = str
"""The unique identifier of the struct parameter field."""


class StructParameterFieldArgument(core.ModelBase):
    """Represents a struct parameter field argument in a logic rule."""

    parameter_id: ParameterId = pydantic.Field(alias=str("parameterId"))  # type: ignore[literal-required]
    struct_parameter_field_api_name: StructParameterFieldApiName = pydantic.Field(alias=str("structParameterFieldApiName"))  # type: ignore[literal-required]
    type: typing.Literal["structParameterFieldValue"] = "structParameterFieldValue"


class StructPropertyMapping(core.ModelBase):
    """A mapping from the backing column struct field names to a struct property's fields."""

    column: ColumnName
    fields: typing.Dict[core_models.StructFieldName, StructFieldPropertyMapping]
    type: typing.Literal["struct"] = "struct"


class StructType(core.ModelBase):
    """StructType"""

    struct_field_types: typing.List[StructFieldType] = pydantic.Field(alias=str("structFieldTypes"))  # type: ignore[literal-required]
    main_value: typing.Optional[StructTypeMainValue] = pydantic.Field(alias=str("mainValue"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["struct"] = "struct"


class StructTypeMainValue(core.ModelBase):
    """StructTypeMainValue"""

    main_value_type: ObjectPropertyType = pydantic.Field(alias=str("mainValueType"))  # type: ignore[literal-required]
    fields: typing.List[StructFieldApiName]
    """The fields which comprise the main value of the struct."""


class SubmissionCriteriaEvaluation(core.ModelBase):
    """
    Contains the status of the **submission criteria**.
    **Submission criteria** are the prerequisites that need to be satisfied before an Action can be applied.
    These are configured in the **Ontology Manager**.
    """

    configured_failure_message: typing.Optional[str] = pydantic.Field(alias=str("configuredFailureMessage"), default=None)  # type: ignore[literal-required]
    """
    The message indicating one of the **submission criteria** was not satisfied.
    This is configured per **submission criteria** in the **Ontology Manager**.
    """

    result: ValidationResult


class SubscriptionClosed(core.ModelBase):
    """The subscription has been closed due to an irrecoverable error during its lifecycle."""

    id: SubscriptionId
    cause: SubscriptionClosureCause
    type: typing.Literal["subscriptionClosed"] = "subscriptionClosed"


SubscriptionClosureCause: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["Reason", "Error"], pydantic.Field(discriminator="type")
]
"""SubscriptionClosureCause"""


class SubscriptionError(core.ModelBase):
    """SubscriptionError"""

    errors: typing.List[Error]
    type: typing.Literal["error"] = "error"


SubscriptionId: typing_extensions.TypeAlias = core.UUID
"""A unique identifier used to associate subscription requests with responses."""


class SubscriptionSuccess(core.ModelBase):
    """SubscriptionSuccess"""

    id: SubscriptionId
    type: typing.Literal["success"] = "success"


class SubtractPropertyExpression(core.ModelBase):
    """Subtracts the right numeric value from the left numeric value."""

    left: DerivedPropertyDefinition
    right: DerivedPropertyDefinition
    type: typing.Literal["subtract"] = "subtract"


class SumAggregationV2(core.ModelBase):
    """
    Computes the sum of values for the provided field.
    Either `field` or `propertyIdentifier` must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    name: typing.Optional[AggregationMetricName] = None
    direction: typing.Optional[OrderByDirection] = None
    type: typing.Literal["sum"] = "sum"


class SyncApplyActionResponseV2(core.ModelBase):
    """SyncApplyActionResponseV2"""

    operation_id: typing.Optional[core.RID] = pydantic.Field(alias=str("operationId"), default=None)  # type: ignore[literal-required]
    validation: typing.Optional[ValidateActionResponseV2] = None
    edits: typing.Optional[ActionResults] = None


class SynchronousWebhookOutputArgument(core.ModelBase):
    """Represents a synchronous webhook output argument in a logic rule."""

    webhook_output_param_name: str = pydantic.Field(alias=str("webhookOutputParamName"))  # type: ignore[literal-required]
    """The name of the webhook output parameter."""

    type: typing.Literal["synchronousWebhookOutput"] = "synchronousWebhookOutput"


TableRid: typing_extensions.TypeAlias = core.RID
"""The RID of a Foundry table."""


class TextAllowedValues(core.ModelBase):
    """
    The parameter value (a string) must satisfy the configured length bounds and/or regex
    pattern.
    """

    gte: typing.Optional[int] = None
    """Character length greater than or equal."""

    lte: typing.Optional[int] = None
    """Character length less than or equal."""

    regex: typing.Optional[str] = None
    """The regular expression. Format and supported syntax match Elasticsearch regex semantics."""

    configured_failure_message: typing.Optional[str] = pydantic.Field(alias=str("configuredFailureMessage"), default=None)  # type: ignore[literal-required]
    """Message returned when the value does not match the pattern."""

    type: typing.Literal["text"] = "text"


class ThreeDimensionalAggregation(core.ModelBase):
    """ThreeDimensionalAggregation"""

    key_type: QueryAggregationKeyType = pydantic.Field(alias=str("keyType"))  # type: ignore[literal-required]
    value_type: TwoDimensionalAggregation = pydantic.Field(alias=str("valueType"))  # type: ignore[literal-required]
    type: typing.Literal["threeDimensionalAggregation"] = "threeDimensionalAggregation"


class TimeCodeFormat(core.ModelBase):
    """Formats the duration in a timecode format."""

    type: typing.Literal["timecode"] = "timecode"


TimeRange: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["AbsoluteTimeRange", "RelativeTimeRange"], pydantic.Field(discriminator="type")
]
"""An absolute or relative range for a time series query."""


TimeSeriesAggregationMethod: typing_extensions.TypeAlias = typing.Literal[
    "SUM",
    "MEAN",
    "STANDARD_DEVIATION",
    "MAX",
    "MIN",
    "PERCENT_CHANGE",
    "DIFFERENCE",
    "PRODUCT",
    "COUNT",
    "FIRST",
    "LAST",
]
"""The aggregation function to use for aggregating time series data."""


TimeSeriesAggregationStrategy: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "TimeSeriesRollingAggregate", "TimeSeriesPeriodicAggregate", "TimeSeriesCumulativeAggregate"
    ],
    pydantic.Field(discriminator="type"),
]
"""
CUMULATIVE aggregates all points up to the current point.
ROLLING aggregates all points in a rolling window whose size is either the specified number of points or
time duration.
PERIODIC aggregates all points in specified time windows.
"""


class TimeSeriesCumulativeAggregate(core.ModelBase):
    """
    The cumulative aggregate is calculated progressively for each point in the input time series,
    considering all preceding points up to and including the current point.
    """

    type: typing.Literal["cumulative"] = "cumulative"


class TimeSeriesPeriodicAggregate(core.ModelBase):
    """
    Aggregates values over discrete, periodic windows for a given time series.

    A periodic window divides the time series into windows of fixed durations.
    For each window, an aggregate function is applied to the points within that window. The result is a time series
    with values representing the aggregate for each window. Windows with no data points are not included
    in the output.

    Periodic aggregation is useful for downsampling a continuous stream of data to larger granularities such as
    hourly, daily, monthly.
    """

    window_size: PreciseDuration = pydantic.Field(alias=str("windowSize"))  # type: ignore[literal-required]
    alignment_timestamp: typing.Optional[core.AwareDatetime] = pydantic.Field(alias=str("alignmentTimestamp"), default=None)  # type: ignore[literal-required]
    """
    The timestamp used to align the result, such that ticks in the result time series will lie at integer
    multiples of the window duration from the alignment timestamp.

    Default is the first epoch timestamp (January 1, 1970, 00:00:00 UTC) so that all aggregated points have
    timestamps at midnight UTC at the start of each window duration.

    For example, for a weekly aggregate with alignment timestamp 5 January, 8:33PM, 
    each aggregated timestamp will lie on the 7 day intervals at 8:33PM starting at 5 January.
    """

    window_type: TimeSeriesWindowType = pydantic.Field(alias=str("windowType"))  # type: ignore[literal-required]
    type: typing.Literal["periodic"] = "periodic"


class TimeSeriesPoint(core.ModelBase):
    """A time and value pair."""

    time: core.AwareDatetime
    """An ISO 8601 timestamp"""

    value: typing.Any
    """An object which is either an enum String or a double number."""


class TimeSeriesRollingAggregate(core.ModelBase):
    """TimeSeriesRollingAggregate"""

    window_size: TimeSeriesRollingAggregateWindow = pydantic.Field(alias=str("windowSize"))  # type: ignore[literal-required]
    type: typing.Literal["rolling"] = "rolling"


TimeSeriesRollingAggregateWindow: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union["PreciseDuration", "RollingAggregateWindowPoints"],
    pydantic.Field(discriminator="type"),
]
"""
A rolling window is a moving subset of data points that ends at the current timestamp (inclusive)
and spans a specified duration (window size). As new data points are added, old points fall out of the
window if they are outside the specified duration.

Rolling windows are commonly used for smoothing data, detecting trends, and reducing noise
in time series analysis.
"""


TimeSeriesWindowType: typing_extensions.TypeAlias = typing.Literal["START", "END"]
"""TimeSeriesWindowType"""


TimeUnit: typing_extensions.TypeAlias = typing.Literal[
    "MILLISECONDS", "SECONDS", "MINUTES", "HOURS", "DAYS", "WEEKS", "MONTHS", "YEARS", "QUARTERS"
]
"""TimeUnit"""


class TimeseriesEntry(core.ModelBase):
    """A time and value pair."""

    time: core.AwareDatetime
    """An ISO 8601 timestamp"""

    value: typing.Any
    """An object which is either an enum String, double number, or a geopoint."""


TimeseriesSyncRid: typing_extensions.TypeAlias = core.RID
"""The RID identifying a time series sync."""


class TimestampValue(core.ModelBase):
    """TimestampValue"""

    value: core.AwareDatetime
    type: typing.Literal["timestampValue"] = "timestampValue"


class TitlePropertySelector(core.ModelBase):
    """Specifies the title property of an object type which is present on all object types."""

    type: typing.Literal["titleProperty"] = "titleProperty"


TransactionEdit: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "ModifyObjectEdit", "DeleteObjectEdit", "AddObjectEdit", "DeleteLinkEdit", "AddLinkEdit"
    ],
    pydantic.Field(discriminator="type"),
]
"""TransactionEdit"""


class TwoDimensionalAggregation(core.ModelBase):
    """TwoDimensionalAggregation"""

    key_type: QueryAggregationKeyType = pydantic.Field(alias=str("keyType"))  # type: ignore[literal-required]
    value_type: QueryAggregationValueType = pydantic.Field(alias=str("valueType"))  # type: ignore[literal-required]
    type: typing.Literal["twoDimensionalAggregation"] = "twoDimensionalAggregation"


class TypeClass(core.ModelBase):
    """Additional metadata that can be interpreted by user applications that interact with the Ontology"""

    kind: str
    """A namespace for the type class."""

    name: str
    """The value of the type class."""


class TypeClassPredicateV2(core.ModelBase):
    """
    A predicate for matching type classes. Matches a type class when `kind`, and `name` if provided, match the
    corresponding attribute of the type class. If `name` is empty, only `kind` is required to match. You can
    search for both parameter type classes and action type type classes.
    """

    kind: str
    """Exact match predicate for the `kind` attribute of a type class."""

    name: typing.Optional[str] = None
    """Exact match predicate for the `name` attribute of a type class."""


class TypeClassesActionTypesQueryV2(core.ModelBase):
    """Returns action types whose type class satisfies the given type class predicate."""

    value: TypeClassPredicateV2
    type: typing.Literal["typeClasses"] = "typeClasses"


TypeReferenceIdentifier: typing_extensions.TypeAlias = str
"""
The unique identifier of a type reference. This identifier is used to look up the
type definition in the `typeReferences` map of the enclosing Query.
"""


class UnevaluableConstraint(core.ModelBase):
    """
    The parameter cannot be evaluated because it depends on another parameter or object set that can't be evaluated.
    This can happen when a parameter's allowed values are defined by another parameter that is missing or invalid.
    """

    type: typing.Literal["unevaluable"] = "unevaluable"


class UniqueIdentifierArgument(core.ModelBase):
    """Represents a unique identifier argument in a logic rule."""

    link_id: typing.Optional[core.UUID] = pydantic.Field(alias=str("linkId"), default=None)  # type: ignore[literal-required]
    """
    By default all UniqueIdentifier Logic Rule arguments will generate different UUID. 
    If the linkId is present all Logic Rules with the same linkId will all have the same uuid generated as the value.
    """

    type: typing.Literal["uniqueIdentifier"] = "uniqueIdentifier"


UniqueIdentifierLinkId: typing_extensions.TypeAlias = core.UUID
"""A reference to a UniqueIdentifierArgument linkId defined for this action type."""


UniqueIdentifierValue: typing_extensions.TypeAlias = core.UUID
"""
An override value to be used for a UniqueIdentifier action parameter, instead of 
the value being automatically generated.
"""


class UnsupportedPolicy(core.ModelBase):
    """Indicates the property is backed by a restricted view that does not support property securities."""

    type: typing.Literal["unsupportedPolicy"] = "unsupportedPolicy"


class UuidConstraint(core.ModelBase):
    """The string must be a valid UUID (Universally Unique Identifier)."""

    type: typing.Literal["uuid"] = "uuid"


class ValidateActionResponseV2(core.ModelBase):
    """ValidateActionResponseV2"""

    result: ValidationResult
    submission_criteria: typing.List[SubmissionCriteriaEvaluation] = pydantic.Field(alias=str("submissionCriteria"))  # type: ignore[literal-required]
    parameters: typing.Dict[ParameterId, ParameterEvaluationResult]


ValidationResult: typing_extensions.TypeAlias = typing.Literal["VALID", "INVALID"]
"""Represents the state of a validation."""


ValueType: typing_extensions.TypeAlias = str
"""
A string indicating the type of each data value. Note that these types can be nested, for example an array of
structs.

| Type                | JSON value                                                                                                        |
|---------------------|-------------------------------------------------------------------------------------------------------------------|
| Array               | `Array<T>`, where `T` is the type of the array elements, e.g. `Array<String>`.                                    |
| Attachment          | `Attachment`                                                                                                      |
| Boolean             | `Boolean`                                                                                                         |
| Byte                | `Byte`                                                                                                            |
| CipherText          | `CipherText`                                                                                                      |
| Date                | `LocalDate`                                                                                                       |
| Decimal             | `Decimal`                                                                                                         |
| Double              | `Double`                                                                                                          |
| Float               | `Float`                                                                                                           |
| Integer             | `Integer`                                                                                                         |
| Long                | `Long`                                                                                                            |
| Marking             | `Marking`                                                                                                         |
| OntologyObject      | `OntologyObject<T>` where `T` is the API name of the referenced object type.                                      |
| Short               | `Short`                                                                                                           |
| String              | `String`                                                                                                          |
| Struct              | `Struct<T>` where `T` contains field name and type pairs, e.g. `Struct<{ firstName: String, lastName: string }>`  |
| Timeseries          | `TimeSeries<T>` where `T` is either `String` for an enum series or `Double` for a numeric series.                 |
| Timestamp           | `Timestamp`                                                                                                       |
"""


class ValueTypeAllowedValues(core.ModelBase):
    """The parameter value must conform to the referenced value type."""

    api_name: ValueTypeApiName = pydantic.Field(alias=str("apiName"))  # type: ignore[literal-required]
    rid: ValueTypeRid
    version_id: ValueTypeVersionId = pydantic.Field(alias=str("versionId"))  # type: ignore[literal-required]
    type: typing.Literal["valueType"] = "valueType"


ValueTypeApiName: typing_extensions.TypeAlias = str
"""The name of the value type in the API in camelCase format."""


class ValueTypeArrayType(core.ModelBase):
    """ValueTypeArrayType"""

    sub_type: typing.Optional[ValueTypeFieldType] = pydantic.Field(alias=str("subType"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["array"] = "array"


ValueTypeConstraint: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        "StructConstraint",
        "RegexConstraint",
        core_models.UnsupportedType,
        "ArrayConstraint",
        "LengthConstraint",
        "RangesConstraint",
        "RidConstraint",
        "UuidConstraint",
        "EnumConstraint",
    ],
    pydantic.Field(discriminator="type"),
]
"""ValueTypeConstraint"""


class ValueTypeDecimalType(core.ModelBase):
    """ValueTypeDecimalType"""

    type: typing.Literal["decimal"] = "decimal"


ValueTypeFieldType: typing_extensions.TypeAlias = typing_extensions.Annotated[
    typing.Union[
        core_models.DateType,
        "ValueTypeStructType",
        core_models.StringType,
        core_models.ByteType,
        core_models.DoubleType,
        "ValueTypeOptionalType",
        core_models.IntegerType,
        "ValueTypeUnionType",
        core_models.FloatType,
        core_models.LongType,
        "ValueTypeReferenceType",
        core_models.BooleanType,
        "ValueTypeArrayType",
        core_models.BinaryType,
        core_models.ShortType,
        "ValueTypeDecimalType",
        "ValueTypeMapType",
        core_models.TimestampType,
    ],
    pydantic.Field(discriminator="type"),
]
"""ValueTypeFieldType"""


class ValueTypeMapType(core.ModelBase):
    """ValueTypeMapType"""

    key_type: typing.Optional[ValueTypeFieldType] = pydantic.Field(alias=str("keyType"), default=None)  # type: ignore[literal-required]
    value_type: typing.Optional[ValueTypeFieldType] = pydantic.Field(alias=str("valueType"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["map"] = "map"


class ValueTypeOptionalType(core.ModelBase):
    """ValueTypeOptionalType"""

    wrapped_type: typing.Optional[ValueTypeFieldType] = pydantic.Field(alias=str("wrappedType"), default=None)  # type: ignore[literal-required]
    type: typing.Literal["optional"] = "optional"


class ValueTypeReferenceType(core.ModelBase):
    """ValueTypeReferenceType"""

    type: typing.Literal["reference"] = "reference"


ValueTypeRid: typing_extensions.TypeAlias = core.RID
"""ValueTypeRid"""


ValueTypeStatus: typing_extensions.TypeAlias = typing.Literal["ACTIVE", "DEPRECATED"]
"""ValueTypeStatus"""


class ValueTypeStructField(core.ModelBase):
    """ValueTypeStructField"""

    name: typing.Optional[core_models.StructFieldName] = None
    field_type: typing.Optional[ValueTypeFieldType] = pydantic.Field(alias=str("fieldType"), default=None)  # type: ignore[literal-required]


class ValueTypeStructType(core.ModelBase):
    """ValueTypeStructType"""

    fields: typing.List[ValueTypeStructField]
    type: typing.Literal["struct"] = "struct"


class ValueTypeUnionType(core.ModelBase):
    """ValueTypeUnionType"""

    member_types: typing.List[ValueTypeFieldType] = pydantic.Field(alias=str("memberTypes"))  # type: ignore[literal-required]
    type: typing.Literal["union"] = "union"


ValueTypeVersionId: typing_extensions.TypeAlias = core.UUID
"""ValueTypeVersionId"""


VersionedQueryTypeApiName: typing_extensions.TypeAlias = str
"""
The name of the Query in the API and an optional version identifier separated by a colon.
If the API name contains a colon, then a version identifier of either "latest" or a semantic version must
be included.
If the API does not contain a colon, then either the version identifier must be excluded or a version
identifier of a semantic version must be included.
Examples: 'myGroup:myFunction:latest', 'myGroup:myFunction:1.0.0', 'myFunction', 'myFunction:2.0.0'
"""


WebhookRid: typing_extensions.TypeAlias = core.RID
"""The unique resource identifier of a webhook, useful for interacting with other Foundry APIs."""


class WebhookRidActionTypesQueryV2(core.ModelBase):
    """Returns action types which reference the webhook with the given rid."""

    value: WebhookRid
    type: typing.Literal["webhookRid"] = "webhookRid"


class WildcardQuery(core.ModelBase):
    """
    Returns objects where the specified field matches the wildcard pattern provided.
    Either `field` or `propertyIdentifier` can be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: str
    type: typing.Literal["wildcard"] = "wildcard"


class WithinBoundingBoxQuery(core.ModelBase):
    """
    Returns objects where the specified field contains a point within the bounding box provided. Allows you to
    specify a property to query on by a variety of means. Either `field` or `propertyIdentifier` must be supplied,
    but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: BoundingBoxValue
    type: typing.Literal["withinBoundingBox"] = "withinBoundingBox"


class WithinDistanceOfQuery(core.ModelBase):
    """
    Returns objects where the specified field contains a point within the distance provided of the center point.
    Allows you to specify a property to query on by a variety of means. Either `field` or `propertyIdentifier`
    must be supplied, but not both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: CenterPoint
    type: typing.Literal["withinDistanceOf"] = "withinDistanceOf"


class WithinPolygonQuery(core.ModelBase):
    """
    Returns objects where the specified field contains a point within the polygon provided. Allows you to specify a
    property to query on by a variety of means. Either `field` or `propertyIdentifier` must be supplied, but not
    both.
    """

    field: typing.Optional[PropertyApiName] = None
    property_identifier: typing.Optional[PropertyIdentifier] = pydantic.Field(alias=str("propertyIdentifier"), default=None)  # type: ignore[literal-required]
    value: PolygonValue
    type: typing.Literal["withinPolygon"] = "withinPolygon"


ArrayEntryEvaluatedConstraint: typing_extensions.TypeAlias = StructEvaluatedConstraint
"""Evaluated constraints for entries of array parameters for which per-entry evaluation is supported."""


CenterPointTypes: typing_extensions.TypeAlias = geo_models.GeoPoint
"""CenterPointTypes"""


Icon: typing_extensions.TypeAlias = BlueprintIcon
"""A union currently only consisting of the BlueprintIcon (more icon types may be added in the future)."""


MethodObjectSet: typing_extensions.TypeAlias = ObjectSet
"""MethodObjectSet"""


OntologyBase: typing_extensions.TypeAlias = OntologyBaseBranch
"""The base used to initialize a scenario."""


PolygonValue: typing_extensions.TypeAlias = geo_models.Polygon
"""PolygonValue"""


ReferenceValue: typing_extensions.TypeAlias = GeotimeSeriesValue
"""Resolved data values pointed to by a reference."""


RelativeDateRangeBound: typing_extensions.TypeAlias = RelativePointInTime
"""Specifies a bound for a relative date range query."""


WithinBoundingBoxPoint: typing_extensions.TypeAlias = geo_models.GeoPoint
"""WithinBoundingBoxPoint"""


core.resolve_forward_references_in_module(__name__)

__all__ = [
    "AbsoluteTimeRange",
    "AbsoluteValuePropertyExpression",
    "ActionExecutionTime",
    "ActionLogicRule",
    "ActionParameterArrayType",
    "ActionParameterRid",
    "ActionParameterType",
    "ActionParameterV2",
    "ActionParameterValidation",
    "ActionParameterValidationBlock",
    "ActionResults",
    "ActionRid",
    "ActionSectionRid",
    "ActionTypeApiName",
    "ActionTypeApiNameActionTypesQueryV2",
    "ActionTypeDescriptionActionTypesQueryV2",
    "ActionTypeDisplayNameActionTypesQueryV2",
    "ActionTypeFullMetadata",
    "ActionTypeFuzziness",
    "ActionTypeLogicRuleTypeFilter",
    "ActionTypePermissionModelFilter",
    "ActionTypeRid",
    "ActionTypeRidActionTypesQueryV2",
    "ActionTypeSearchJsonQueryV2",
    "ActionTypeSortByV2",
    "ActionTypeStatusFilter",
    "ActionTypeV2",
    "ActivePropertyTypeStatus",
    "AddLink",
    "AddLinkEdit",
    "AddObject",
    "AddObjectEdit",
    "AddPropertyExpression",
    "AffectedInterfaceTypeRidActionTypesQueryV2",
    "AffectedLinkTypeRidActionTypesQueryV2",
    "AffectedObjectTypeRidActionTypesQueryV2",
    "Affix",
    "AggregateObjectSetRequestV2",
    "AggregateObjectsRequestV2",
    "AggregateObjectsResponseItemV2",
    "AggregateObjectsResponseV2",
    "AggregateTimeSeries",
    "AggregationAccuracy",
    "AggregationAccuracyRequest",
    "AggregationDurationGroupingV2",
    "AggregationExactGroupingV2",
    "AggregationFixedWidthGroupingV2",
    "AggregationGroupByV2",
    "AggregationGroupKeyV2",
    "AggregationGroupValueV2",
    "AggregationMetricName",
    "AggregationMetricResultV2",
    "AggregationRangeV2",
    "AggregationRangesGroupingV2",
    "AggregationV2",
    "AllOfRule",
    "AndActionTypesQueryV2",
    "AndQueryV2",
    "AnyOfRule",
    "ApplyActionMode",
    "ApplyActionOverrides",
    "ApplyActionRequestOptions",
    "ApplyActionRequestV2",
    "ApplyActionWithOverridesRequest",
    "ApplyReducersAndExtractMainValueLoadLevel",
    "ApplyReducersLoadLevel",
    "ApplyScenarioLogicRule",
    "ApplyScenarioRule",
    "ApproximateDistinctAggregationV2",
    "ApproximatePercentileAggregationV2",
    "Arg",
    "ArrayConstraint",
    "ArrayEntryEvaluatedConstraint",
    "ArrayEvaluatedConstraint",
    "ArraySizeConstraint",
    "ArtifactRepositoryRid",
    "AttachmentAllowedValues",
    "AttachmentMetadataResponse",
    "AttachmentRid",
    "AttachmentV2",
    "AvgAggregationV2",
    "BatchActionObjectEdit",
    "BatchActionObjectEdits",
    "BatchActionResults",
    "BatchApplyActionRequestItem",
    "BatchApplyActionRequestItemWithOverrides",
    "BatchApplyActionRequestOptions",
    "BatchApplyActionRequestV2",
    "BatchApplyActionResponseV2",
    "BatchApplyActionWithOverridesRequest",
    "BatchReturnEditsMode",
    "BatchedFunctionLogicRule",
    "BlueprintIcon",
    "BooleanValue",
    "BoundingBoxValue",
    "CenterPoint",
    "CenterPointTypes",
    "CipherChannelStrategy",
    "CipherText",
    "ColumnName",
    "ColumnPropertyMapping",
    "ConjunctiveMarkingSummary",
    "ContainerConjunctiveMarkingSummary",
    "ContainerDisjunctiveMarkingSummary",
    "ContainsAllTermsInOrderPrefixLastTerm",
    "ContainsAllTermsInOrderQuery",
    "ContainsAllTermsQuery",
    "ContainsAnyTermQuery",
    "ContainsQueryV2",
    "CountAggregationV2",
    "CountObjectsResponseV2",
    "CreateEdit",
    "CreateInterfaceLinkLogicRule",
    "CreateInterfaceLogicRule",
    "CreateInterfaceObjectRule",
    "CreateLinkLogicRule",
    "CreateLinkRule",
    "CreateObjectLogicRule",
    "CreateObjectRule",
    "CreateOntologyScenarioRequest",
    "CreateOntologyScenarioResponse",
    "CreateOrModifyObjectLogicRule",
    "CreateOrModifyObjectLogicRuleV2",
    "CreateTemporaryObjectSetRequestV2",
    "CreateTemporaryObjectSetResponseV2",
    "CurrentTimeArgument",
    "CurrentUserArgument",
    "DataValue",
    "DatasourceBranchId",
    "DatasourceRid",
    "DateValue",
    "DatetimeAllowedValues",
    "DatetimeFormat",
    "DatetimeLocalizedFormat",
    "DatetimeLocalizedFormatType",
    "DatetimeStringFormat",
    "DatetimeTimezone",
    "DatetimeTimezoneStatic",
    "DatetimeTimezoneUser",
    "DecryptionResult",
    "DeleteEdit",
    "DeleteInterfaceLinkLogicRule",
    "DeleteInterfaceObjectRule",
    "DeleteLink",
    "DeleteLinkEdit",
    "DeleteLinkLogicRule",
    "DeleteLinkRule",
    "DeleteObject",
    "DeleteObjectEdit",
    "DeleteObjectLogicRule",
    "DeleteObjectRule",
    "DeprecatedPropertyTypeStatus",
    "DerivedPropertyApiName",
    "DerivedPropertyDefinition",
    "DirectSourceRid",
    "DisjunctiveMarkingSummary",
    "DividePropertyExpression",
    "DoesNotIntersectBoundingBoxQuery",
    "DoesNotIntersectPolygonQuery",
    "DoubleValue",
    "DoubleVector",
    "DurationBaseValue",
    "DurationFormatStyle",
    "DurationPrecision",
    "EditHistoryEdit",
    "EditOnlyPropertyMapping",
    "EditsHistoryFilter",
    "EditsHistoryOperationIdsFilter",
    "EditsHistorySortOrder",
    "EditsHistoryTimestampFilter",
    "EncryptionRequest",
    "EncryptionResult",
    "EntrySetType",
    "EnumConstraint",
    "EqualsQueryV2",
    "Error",
    "ErrorComputingSecurity",
    "ErrorName",
    "ExactDistinctAggregationV2",
    "ExamplePropertyTypeStatus",
    "ExecuteQueryRequest",
    "ExecuteQueryResponse",
    "ExperimentalPropertyTypeStatus",
    "ExtractDatePart",
    "ExtractMainValueLoadLevel",
    "ExtractPropertyExpression",
    "FilterValue",
    "FixedDatetimeValue",
    "FixedValuesMapKey",
    "FullTextStringContainsPredicate",
    "FullTextStringExactPredicate",
    "FullTextStringPredicateV2",
    "FunctionLogicRule",
    "FunctionParameterName",
    "FunctionRid",
    "FunctionRidActionTypesQueryV2",
    "FunctionVersion",
    "FuzzinessAuto",
    "FuzzinessOff",
    "FuzzyRule",
    "FuzzyV2",
    "GeoJsonString",
    "GeoShapeV2Geometry",
    "GeoShapeV2Query",
    "GeotemporalSeriesEntry",
    "GeotimeSeriesIntegrationRid",
    "GeotimeSeriesValue",
    "GetActionTypeByRidBatchRequest",
    "GetActionTypeByRidBatchRequestElement",
    "GetActionTypeByRidBatchResponse",
    "GetActionTypeFullMetadataBatchRequest",
    "GetActionTypeFullMetadataBatchRequestElement",
    "GetActionTypeFullMetadataBatchResponse",
    "GetObjectTypeByRidBatchRequest",
    "GetObjectTypeByRidBatchRequestElement",
    "GetObjectTypeByRidBatchResponse",
    "GetObjectTypeFullMetadataBatchRequest",
    "GetObjectTypeFullMetadataBatchRequestElement",
    "GetObjectTypeFullMetadataBatchResponse",
    "GetOutgoingLinkTypesByObjectTypeRidBatchRequest",
    "GetOutgoingLinkTypesByObjectTypeRidBatchRequestElement",
    "GetOutgoingLinkTypesByObjectTypeRidBatchResponse",
    "GetQueryTypeByRidBatchRequest",
    "GetQueryTypeByRidBatchRequestElement",
    "GetQueryTypeByRidBatchResponse",
    "GetSelectedPropertyOperation",
    "GreatestPropertyExpression",
    "GroupMemberConstraint",
    "GtQueryV2",
    "GteQueryV2",
    "HasActionLogActionTypesQueryV2",
    "HasNotificationActionTypesQueryV2",
    "HasWebhookActionTypesQueryV2",
    "HumanReadableFormat",
    "Icon",
    "InQuery",
    "InputObjectTypeRidActionTypesQueryV2",
    "IntegerValue",
    "InterfaceActionTypeConstraintApiName",
    "InterfaceDefinedPropertyType",
    "InterfaceLinkType",
    "InterfaceLinkTypeApiName",
    "InterfaceLinkTypeCardinality",
    "InterfaceLinkTypeLinkedEntityApiName",
    "InterfaceLinkTypeRid",
    "InterfaceParameterPropertyArgument",
    "InterfacePropertyApiName",
    "InterfacePropertyLocalPropertyImplementation",
    "InterfacePropertyReducedPropertyImplementation",
    "InterfacePropertyStructFieldImplementation",
    "InterfacePropertyStructImplementation",
    "InterfacePropertyStructImplementationMapping",
    "InterfacePropertyType",
    "InterfacePropertyTypeImplementation",
    "InterfacePropertyTypeRid",
    "InterfaceSharedPropertyType",
    "InterfaceToObjectTypeMapping",
    "InterfaceToObjectTypeMappingV2",
    "InterfaceToObjectTypeMappings",
    "InterfaceToObjectTypeMappingsV2",
    "InterfaceType",
    "InterfaceTypeApiName",
    "InterfaceTypeRid",
    "IntersectsBoundingBoxQuery",
    "IntersectsPolygonQuery",
    "IntervalQuery",
    "IntervalQueryRule",
    "IsNullQueryV2",
    "KnownType",
    "LeastPropertyExpression",
    "LengthConstraint",
    "LinkSideObject",
    "LinkTypeApiName",
    "LinkTypeId",
    "LinkTypeRid",
    "LinkTypeSideCardinality",
    "LinkTypeSideV2",
    "LinkedInterfaceTypeApiName",
    "LinkedObjectLocator",
    "LinkedObjectTypeApiName",
    "LinksFromObject",
    "ListActionTypesFullMetadataResponse",
    "ListActionTypesResponseV2",
    "ListAttachmentsResponseV2",
    "ListInterfaceLinkedObjectsResponse",
    "ListInterfaceTypesResponse",
    "ListLinkedObjectsResponseV2",
    "ListObjectTypesV2Response",
    "ListObjectsForInterfaceResponse",
    "ListObjectsResponseV2",
    "ListOntologiesV2Response",
    "ListOntologyValueTypesResponse",
    "ListOutgoingInterfaceLinkTypesResponse",
    "ListOutgoingLinkTypesResponseV2",
    "ListQueryTypesResponseV2",
    "ListScenarioConflictingObjectsResponse",
    "ListScenarioEditedEntityTypesResponse",
    "ListScenarioEditedLinkTypesResponse",
    "ListScenarioEditedLinksResponse",
    "ListScenarioEditedObjectTypesResponse",
    "ListScenarioEditedObjectsResponse",
    "LoadGeotemporalSeriesRequest",
    "LoadGeotemporalSeriesResponse",
    "LoadObjectSetLinksRequestV2",
    "LoadObjectSetLinksResponseV2",
    "LoadObjectSetRequestV2",
    "LoadObjectSetResponseV2",
    "LoadObjectSetV2MultipleObjectTypesRequest",
    "LoadObjectSetV2MultipleObjectTypesResponse",
    "LoadObjectSetV2ObjectsOrInterfacesRequest",
    "LoadObjectSetV2ObjectsOrInterfacesResponse",
    "LoadOntologyMetadataRequest",
    "LogicRule",
    "LogicRuleActionTypesQueryV2",
    "LogicRuleArgument",
    "LongValue",
    "LtQueryV2",
    "LteQueryV2",
    "MarkdownAllowedValues",
    "MarkingId",
    "MatchRule",
    "MaxAggregationV2",
    "MediaMetadata",
    "MethodObjectSet",
    "MinAggregationV2",
    "ModifyEdit",
    "ModifyInterfaceLogicRule",
    "ModifyInterfaceObjectRule",
    "ModifyObject",
    "ModifyObjectEdit",
    "ModifyObjectLogicRule",
    "ModifyObjectRule",
    "MultiplyPropertyExpression",
    "MustBeEmptyAllowedValues",
    "NearestNeighborsQuery",
    "NearestNeighborsQueryText",
    "NegatePropertyExpression",
    "NestedInterfacePropertyTypeImplementation",
    "NestedQueryAggregation",
    "NoLoadLevel",
    "NotQueryV2",
    "NowDatetimeValue",
    "NumberFormatAffix",
    "NumberFormatCurrency",
    "NumberFormatCurrencyStyle",
    "NumberFormatCustomUnit",
    "NumberFormatDuration",
    "NumberFormatFixedValues",
    "NumberFormatNotation",
    "NumberFormatOptions",
    "NumberFormatRatio",
    "NumberFormatScale",
    "NumberFormatStandard",
    "NumberFormatStandardUnit",
    "NumberRatioType",
    "NumberRoundingMode",
    "NumberScaleType",
    "ObjectEdit",
    "ObjectEditHistoryEntry",
    "ObjectEdits",
    "ObjectLoadingResponseOptions",
    "ObjectLocator",
    "ObjectParameterPropertyArgument",
    "ObjectPrimaryKey",
    "ObjectPrimaryKeyV2",
    "ObjectPropertyType",
    "ObjectPropertyValueConstraint",
    "ObjectQueryResultConstraint",
    "ObjectRid",
    "ObjectSet",
    "ObjectSetAsBaseObjectTypesType",
    "ObjectSetAsTypeType",
    "ObjectSetBaseType",
    "ObjectSetFilterType",
    "ObjectSetInterfaceBaseType",
    "ObjectSetInterfaceLinkSearchAroundType",
    "ObjectSetIntersectionType",
    "ObjectSetMethodInputType",
    "ObjectSetNearestNeighborsType",
    "ObjectSetReferenceType",
    "ObjectSetRid",
    "ObjectSetSearchAroundType",
    "ObjectSetStaticType",
    "ObjectSetStreamSubscribeRequest",
    "ObjectSetStreamSubscribeRequests",
    "ObjectSetSubscribeResponse",
    "ObjectSetSubscribeResponses",
    "ObjectSetSubtractType",
    "ObjectSetUnionType",
    "ObjectSetUpdate",
    "ObjectSetUpdates",
    "ObjectSetWithPropertiesType",
    "ObjectState",
    "ObjectTypeApiName",
    "ObjectTypeDatasetDatasource",
    "ObjectTypeDatasource",
    "ObjectTypeDatasourceDefinition",
    "ObjectTypeDirectDatasource",
    "ObjectTypeEdits",
    "ObjectTypeEditsHistoryRequest",
    "ObjectTypeEditsHistoryResponse",
    "ObjectTypeEditsOnlyDatasource",
    "ObjectTypeFullMetadata",
    "ObjectTypeGeotimeSeriesDatasource",
    "ObjectTypeId",
    "ObjectTypeInterfaceImplementation",
    "ObjectTypeLinkTypeApiNameMapping",
    "ObjectTypeMediaSetViewDatasource",
    "ObjectTypeRestrictedViewDatasource",
    "ObjectTypeRid",
    "ObjectTypeStreamDatasource",
    "ObjectTypeTableDatasource",
    "ObjectTypeTimeSeriesDatasource",
    "ObjectTypeUnsupportedDatasource",
    "ObjectTypeV2",
    "ObjectTypeVisibility",
    "ObjectUpdate",
    "OneOfAllowedValues",
    "OneOfConstraint",
    "OntologyApiName",
    "OntologyArrayType",
    "OntologyBase",
    "OntologyBaseBranch",
    "OntologyDataType",
    "OntologyFullMetadata",
    "OntologyIdentifier",
    "OntologyInterfaceObjectSetType",
    "OntologyInterfaceObjectType",
    "OntologyMapType",
    "OntologyObjectArrayType",
    "OntologyObjectArrayTypeReducer",
    "OntologyObjectArrayTypeReducerSortDirection",
    "OntologyObjectSetType",
    "OntologyObjectType",
    "OntologyObjectTypeReferenceType",
    "OntologyObjectV2",
    "OntologyRid",
    "OntologyScenarioRid",
    "OntologySetType",
    "OntologyStructField",
    "OntologyStructType",
    "OntologyTransactionId",
    "OntologyV2",
    "OntologyValueType",
    "OrActionTypesQueryV2",
    "OrQueryV2",
    "OrderBy",
    "OrderByDirection",
    "ParameterAllowedValueOption",
    "ParameterAllowedValues",
    "ParameterArraySize",
    "ParameterConstraintValue",
    "ParameterDatetimeValue",
    "ParameterEvaluatedConstraint",
    "ParameterEvaluationResult",
    "ParameterId",
    "ParameterIdArgument",
    "ParameterLengthConstraintValue",
    "ParameterNameActionTypesQueryV2",
    "ParameterOption",
    "ParameterRidActionTypesQueryV2",
    "ParameterValueConstraintValue",
    "PermissionModelActionTypesQueryV2",
    "Plaintext",
    "PolygonValue",
    "PostTransactionEditsRequest",
    "PostTransactionEditsResponse",
    "PreciseDuration",
    "PreciseTimeUnit",
    "PrefixOnLastTokenRule",
    "PrimaryKeyPropertySelector",
    "PrimaryKeyValue",
    "PrimaryKeyValueV2",
    "PropertyApiName",
    "PropertyApiNameSelector",
    "PropertyBooleanFormattingRule",
    "PropertyDateFormattingRule",
    "PropertyFilter",
    "PropertyId",
    "PropertyIdentifier",
    "PropertyImplementation",
    "PropertyKnownTypeFormattingRule",
    "PropertyLoadLevel",
    "PropertyMarkingSummary",
    "PropertyNumberFormattingRule",
    "PropertyNumberFormattingRuleType",
    "PropertyOrStructFieldOfPropertyImplementation",
    "PropertySecurities",
    "PropertySecurity",
    "PropertyTimestampFormattingRule",
    "PropertyTypeApiName",
    "PropertyTypeMappingInfo",
    "PropertyTypeReference",
    "PropertyTypeReferenceOrStringConstant",
    "PropertyTypeRid",
    "PropertyTypeStatus",
    "PropertyTypeVisibility",
    "PropertyV2",
    "PropertyValue",
    "PropertyValueEscapedString",
    "PropertyValueFormattingRule",
    "PropertyWithLoadLevelSelector",
    "QosError",
    "QueryAggregation",
    "QueryAggregationKeyType",
    "QueryAggregationRangeSubType",
    "QueryAggregationRangeType",
    "QueryAggregationValueType",
    "QueryApiName",
    "QueryArrayType",
    "QueryDataType",
    "QueryParameterV2",
    "QueryRuntimeErrorParameter",
    "QuerySetType",
    "QueryStructField",
    "QueryStructType",
    "QueryThreeDimensionalAggregation",
    "QueryTwoDimensionalAggregation",
    "QueryTypeReferenceType",
    "QueryTypeV2",
    "QueryUnionType",
    "RangeAllowedValues",
    "RangeConstraint",
    "RangesConstraint",
    "Reason",
    "ReasonType",
    "ReferenceSigningOptions",
    "ReferenceUpdate",
    "ReferenceValue",
    "RefreshObjectSet",
    "RegexConstraint",
    "RegexQuery",
    "RelativeDateRangeBound",
    "RelativeDateRangeQuery",
    "RelativeDatetimeDuration",
    "RelativeDatetimeTense",
    "RelativeDatetimeUnit",
    "RelativeDatetimeValue",
    "RelativePointInTime",
    "RelativeTime",
    "RelativeTimeRange",
    "RelativeTimeRelation",
    "RelativeTimeSeriesTimeUnit",
    "RelativeTimeUnit",
    "RequestId",
    "ResolvedInterfacePropertyType",
    "RestrictedViewRid",
    "ReturnEditsMode",
    "RevertActionEnabledActionTypesQueryV2",
    "RidConstraint",
    "RollingAggregateWindowPoints",
    "SdkPackageName",
    "SdkPackageRid",
    "SdkVersion",
    "SearchActionTypesFullMetadataRequest",
    "SearchActionTypesFullMetadataResponse",
    "SearchActionTypesOrderByV2",
    "SearchActionTypesRequestV2",
    "SearchActionTypesResponseV2",
    "SearchJsonQueryV2",
    "SearchObjectsForInterfaceRequest",
    "SearchObjectsRequestV2",
    "SearchObjectsResponseV2",
    "SearchOrderByType",
    "SearchOrderByV2",
    "SearchOrderingV2",
    "SectionRidActionTypesQueryV2",
    "SelectedPropertyApiName",
    "SelectedPropertyApproximateDistinctAggregation",
    "SelectedPropertyApproximatePercentileAggregation",
    "SelectedPropertyAvgAggregation",
    "SelectedPropertyCollectListAggregation",
    "SelectedPropertyCollectSetAggregation",
    "SelectedPropertyCountAggregation",
    "SelectedPropertyExactDistinctAggregation",
    "SelectedPropertyExpression",
    "SelectedPropertyMaxAggregation",
    "SelectedPropertyMinAggregation",
    "SelectedPropertyOperation",
    "SelectedPropertySumAggregation",
    "SharedPropertyType",
    "SharedPropertyTypeApiName",
    "SharedPropertyTypeRid",
    "SpatialFilterMode",
    "StartsWithQuery",
    "StaticArgument",
    "StaticConstraintValue",
    "StatusActionTypesQueryV2",
    "StreamMessage",
    "StreamRid",
    "StreamTimeSeriesPointsRequest",
    "StreamTimeSeriesValuesRequest",
    "StreamingOutputFormat",
    "StringConstant",
    "StringLengthConstraint",
    "StringRegexMatchConstraint",
    "StringValue",
    "StructConstraint",
    "StructEvaluatedConstraint",
    "StructFieldApiName",
    "StructFieldArgument",
    "StructFieldEvaluatedConstraint",
    "StructFieldEvaluationResult",
    "StructFieldOfPropertyImplementation",
    "StructFieldPropertyMapping",
    "StructFieldSelector",
    "StructFieldType",
    "StructFieldTypeRid",
    "StructListParameterFieldArgument",
    "StructParameterFieldApiName",
    "StructParameterFieldArgument",
    "StructPropertyMapping",
    "StructType",
    "StructTypeMainValue",
    "SubmissionCriteriaEvaluation",
    "SubscriptionClosed",
    "SubscriptionClosureCause",
    "SubscriptionError",
    "SubscriptionId",
    "SubscriptionSuccess",
    "SubtractPropertyExpression",
    "SumAggregationV2",
    "SyncApplyActionResponseV2",
    "SynchronousWebhookOutputArgument",
    "TableRid",
    "TextAllowedValues",
    "ThreeDimensionalAggregation",
    "TimeCodeFormat",
    "TimeRange",
    "TimeSeriesAggregationMethod",
    "TimeSeriesAggregationStrategy",
    "TimeSeriesCumulativeAggregate",
    "TimeSeriesPeriodicAggregate",
    "TimeSeriesPoint",
    "TimeSeriesRollingAggregate",
    "TimeSeriesRollingAggregateWindow",
    "TimeSeriesWindowType",
    "TimeUnit",
    "TimeseriesEntry",
    "TimeseriesSyncRid",
    "TimestampValue",
    "TitlePropertySelector",
    "TransactionEdit",
    "TwoDimensionalAggregation",
    "TypeClass",
    "TypeClassPredicateV2",
    "TypeClassesActionTypesQueryV2",
    "TypeReferenceIdentifier",
    "UnevaluableConstraint",
    "UniqueIdentifierArgument",
    "UniqueIdentifierLinkId",
    "UniqueIdentifierValue",
    "UnsupportedPolicy",
    "UuidConstraint",
    "ValidateActionResponseV2",
    "ValidationResult",
    "ValueType",
    "ValueTypeAllowedValues",
    "ValueTypeApiName",
    "ValueTypeArrayType",
    "ValueTypeConstraint",
    "ValueTypeDecimalType",
    "ValueTypeFieldType",
    "ValueTypeMapType",
    "ValueTypeOptionalType",
    "ValueTypeReferenceType",
    "ValueTypeRid",
    "ValueTypeStatus",
    "ValueTypeStructField",
    "ValueTypeStructType",
    "ValueTypeUnionType",
    "ValueTypeVersionId",
    "VersionedQueryTypeApiName",
    "WebhookRid",
    "WebhookRidActionTypesQueryV2",
    "WildcardQuery",
    "WithinBoundingBoxPoint",
    "WithinBoundingBoxQuery",
    "WithinDistanceOfQuery",
    "WithinPolygonQuery",
]
