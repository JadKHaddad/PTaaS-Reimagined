// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'models_2.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

APIResponse<T> _$APIResponseFromJson<T>(
  Map<String, dynamic> json,
  T Function(Object? json) fromJsonT,
) =>
    APIResponse<T>(
      processed: _$nullableGenericFromJson(json['processed'], fromJsonT),
      missingToken: json['missingToken'] as bool?,
      emptyToken: json['emptyToken'] as bool?,
      notLoggedIn: json['notLoggedIn'] as bool?,
      internalServerError: json['internalServerError'] as bool?,
    );

Map<String, dynamic> _$APIResponseToJson<T>(
  APIResponse<T> instance,
  Object? Function(T value) toJsonT,
) =>
    <String, dynamic>{
      'processed': _$nullableGenericToJson(instance.processed, toJsonT),
      'missingToken': instance.missingToken,
      'emptyToken': instance.emptyToken,
      'notLoggedIn': instance.notLoggedIn,
      'internalServerError': instance.internalServerError,
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

AllProjectsResponse _$AllProjectsResponseFromJson(Map<String, dynamic> json) =>
    AllProjectsResponse(
      processed: json['processed'] == null
          ? null
          : AllProjectsResponseProcessed.fromJson(
              json['processed'] as Map<String, dynamic>),
      cantReadProjects: json['cantReadProjects'] as bool?,
      aProjectIsMissing: json['aProjectIsMissing'] as bool?,
    );

Map<String, dynamic> _$AllProjectsResponseToJson(
        AllProjectsResponse instance) =>
    <String, dynamic>{
      'processed': instance.processed,
      'cantReadProjects': instance.cantReadProjects,
      'aProjectIsMissing': instance.aProjectIsMissing,
    };

AllProjectsResponseProcessed _$AllProjectsResponseProcessedFromJson(
        Map<String, dynamic> json) =>
    AllProjectsResponseProcessed(
      projects: (json['projects'] as List<dynamic>)
          .map((e) => Project.fromJson(e as Map<String, dynamic>))
          .toList(),
    );

Map<String, dynamic> _$AllProjectsResponseProcessedToJson(
        AllProjectsResponseProcessed instance) =>
    <String, dynamic>{
      'projects': instance.projects,
    };

Script _$ScriptFromJson(Map<String, dynamic> json) => Script(
      id: json['id'] as String,
    );

Map<String, dynamic> _$ScriptToJson(Script instance) => <String, dynamic>{
      'id': instance.id,
    };

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
