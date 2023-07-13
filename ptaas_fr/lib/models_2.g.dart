// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'models_2.dart';

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

APIResponse _$APIResponseFromJson(Map<String, dynamic> json) => APIResponse(
      processed: json['processed'] == null
          ? null
          : APIResponseProcessd.fromJson(
              json['processed'] as Map<String, dynamic>),
      failed: $enumDecodeNullable(_$APIResponseFailedEnumMap, json['failed']),
    );

Map<String, dynamic> _$APIResponseToJson(APIResponse instance) =>
    <String, dynamic>{
      'processed': instance.processed,
      'failed': _$APIResponseFailedEnumMap[instance.failed],
    };

const _$APIResponseFailedEnumMap = {
  APIResponseFailed.missingToken: 'missingToken',
  APIResponseFailed.enmtptyToken: 'enmtptyToken',
  APIResponseFailed.notLoggedIn: 'notLoggedIn',
  APIResponseFailed.internalServerError: 'internalServerError',
};

APIResponseProcessd _$APIResponseProcessdFromJson(Map<String, dynamic> json) =>
    APIResponseProcessd()
      ..allProjects = json['allProjects'] == null
          ? null
          : AllProjectsResponse.fromJson(
              json['allProjects'] as Map<String, dynamic>)
      ..allScripts = json['allScripts'] == null
          ? null
          : AllScriptsResponse.fromJson(
              json['allScripts'] as Map<String, dynamic>);

Map<String, dynamic> _$APIResponseProcessdToJson(
        APIResponseProcessd instance) =>
    <String, dynamic>{
      'allProjects': instance.allProjects,
      'allScripts': instance.allScripts,
    };

AllProjectsResponse _$AllProjectsResponseFromJson(Map<String, dynamic> json) =>
    AllProjectsResponse(
      processed: json['processed'] == null
          ? null
          : AllProjectsResponseProcessed.fromJson(
              json['processed'] as Map<String, dynamic>),
      failed: $enumDecodeNullable(
          _$AllProjectsResponseFailedEnumMap, json['failed']),
    );

Map<String, dynamic> _$AllProjectsResponseToJson(
        AllProjectsResponse instance) =>
    <String, dynamic>{
      'processed': instance.processed,
      'failed': _$AllProjectsResponseFailedEnumMap[instance.failed],
    };

const _$AllProjectsResponseFailedEnumMap = {
  AllProjectsResponseFailed.cantReadProjects: 'cantReadProjects',
  AllProjectsResponseFailed.aProjectIsMissing: 'aProjectIsMissing',
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

AllScriptsResponse _$AllScriptsResponseFromJson(Map<String, dynamic> json) =>
    AllScriptsResponse(
      processed: json['processed'] == null
          ? null
          : AllScriptsResponseProcessed.fromJson(
              json['processed'] as Map<String, dynamic>),
      failed: $enumDecodeNullable(
          _$AllScriptsResponseFailedEnumMap, json['failed']),
    );

Map<String, dynamic> _$AllScriptsResponseToJson(AllScriptsResponse instance) =>
    <String, dynamic>{
      'processed': instance.processed,
      'failed': _$AllScriptsResponseFailedEnumMap[instance.failed],
    };

const _$AllScriptsResponseFailedEnumMap = {
  AllScriptsResponseFailed.cantReadScripts: 'cantReadScripts',
  AllScriptsResponseFailed.aScriptIsMissing: 'aScriptIsMissing',
};

AllScriptsResponseProcessed _$AllScriptsResponseProcessedFromJson(
        Map<String, dynamic> json) =>
    AllScriptsResponseProcessed(
      scripts: (json['scripts'] as List<dynamic>)
          .map((e) => Script.fromJson(e as Map<String, dynamic>))
          .toList(),
    );

Map<String, dynamic> _$AllScriptsResponseProcessedToJson(
        AllScriptsResponseProcessed instance) =>
    <String, dynamic>{
      'scripts': instance.scripts,
    };
