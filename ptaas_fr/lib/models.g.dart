// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'models.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Project _$ProjectFromJson(Map<String, dynamic> json) => Project(
      id: json['id'] as String,
      installed: json['installed'] as bool,
      scripts: (json['scripts'] as List<dynamic>)
          .map((e) => Script.fromJson(e as Map<String, dynamic>))
          .toList(),
    );

Map<String, dynamic> _$ProjectToJson(Project instance) => <String, dynamic>{
      'id': instance.id,
      'installed': instance.installed,
      'scripts': instance.scripts,
    };

Script _$ScriptFromJson(Map<String, dynamic> json) => Script(
      id: json['id'] as String,
    );

Map<String, dynamic> _$ScriptToJson(Script instance) => <String, dynamic>{
      'id': instance.id,
    };

AllScriptsResponse _$AllScriptsResponseFromJson(Map<String, dynamic> json) =>
    AllScriptsResponse(
      scripts: (json['scripts'] as List<dynamic>)
          .map((e) => Script.fromJson(e as Map<String, dynamic>))
          .toList(),
    );

Map<String, dynamic> _$AllScriptsResponseToJson(AllScriptsResponse instance) =>
    <String, dynamic>{
      'scripts': instance.scripts,
    };

AllProjectsResponseData _$AllProjectsResponseDataFromJson(
        Map<String, dynamic> json) =>
    AllProjectsResponseData(
      projects: (json['projects'] as List<dynamic>)
          .map((e) => Project.fromJson(e as Map<String, dynamic>))
          .toList(),
    );

Map<String, dynamic> _$AllProjectsResponseDataToJson(
        AllProjectsResponseData instance) =>
    <String, dynamic>{
      'projects': instance.projects,
    };

APIResponseError<E> _$APIResponseErrorFromJson<E>(
  Map<String, dynamic> json,
  E Function(Object? json) fromJsonE,
) =>
    APIResponseError<E>(
      errorType: fromJsonE(json['errorType']),
      errorMessage: json['errorMessage'] as String,
    );

Map<String, dynamic> _$APIResponseErrorToJson<E>(
  APIResponseError<E> instance,
  Object? Function(E value) toJsonE,
) =>
    <String, dynamic>{
      'errorType': toJsonE(instance.errorType),
      'errorMessage': instance.errorMessage,
    };

APIResponse<D, E> _$APIResponseFromJson<D, E>(
  Map<String, dynamic> json,
  D Function(Object? json) fromJsonD,
  E Function(Object? json) fromJsonE,
) =>
    APIResponse<D, E>(
      success: json['success'] as bool,
      responseType: $enumDecode(_$APIResponseTypeEnumMap, json['responseType']),
      data: _$nullableGenericFromJson(json['data'], fromJsonD),
      error: json['error'] == null
          ? null
          : APIResponseError<E>.fromJson(json['error'] as Map<String, dynamic>,
              (value) => fromJsonE(value)),
    );

Map<String, dynamic> _$APIResponseToJson<D, E>(
  APIResponse<D, E> instance,
  Object? Function(D value) toJsonD,
  Object? Function(E value) toJsonE,
) =>
    <String, dynamic>{
      'success': instance.success,
      'responseType': _$APIResponseTypeEnumMap[instance.responseType]!,
      'data': _$nullableGenericToJson(instance.data, toJsonD),
      'error': instance.error?.toJson(
        (value) => toJsonE(value),
      ),
    };

const _$APIResponseTypeEnumMap = {
  APIResponseType.gerneralResponse: 'gerneralResponse',
  APIResponseType.allProjectsResponse: 'allProjectsResponse',
  APIResponseType.allScriptsResponse: 'allScriptsResponse',
};

T? _$nullableGenericFromJson<T>(
  Object? input,
  T Function(Object? json) fromJson,
) =>
    input == null ? null : fromJson(input);

Object? _$nullableGenericToJson<T>(
  T? input,
  Object? Function(T value) toJson,
) =>
    input == null ? null : toJson(input);
