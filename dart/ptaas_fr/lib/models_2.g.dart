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

APIError _$APIErrorFromJson(Map<String, dynamic> json) => APIError(
      message: json['message'] as String,
      reason: json['reason'] as String?,
    );

Map<String, dynamic> _$APIErrorToJson(APIError instance) => <String, dynamic>{
      'message': instance.message,
      'reason': instance.reason,
    };

APIResponse _$APIResponseFromJson(Map<String, dynamic> json) => APIResponse(
      processed: json['processed'] == null
          ? null
          : APIResponseProcessd.fromJson(
              json['processed'] as Map<String, dynamic>),
      failed: json['failed'] == null
          ? null
          : APIResponseFailed.fromJson(json['failed'] as Map<String, dynamic>),
    );

Map<String, dynamic> _$APIResponseToJson(APIResponse instance) =>
    <String, dynamic>{
      'processed': instance.processed,
      'failed': instance.failed,
    };

APIResponseProcessd _$APIResponseProcessdFromJson(Map<String, dynamic> json) =>
    APIResponseProcessd(
      allProjects: json['allProjects'] == null
          ? null
          : AllProjectsResponse.fromJson(
              json['allProjects'] as Map<String, dynamic>),
      allScripts: json['allScripts'] == null
          ? null
          : AllScriptsResponse.fromJson(
              json['allScripts'] as Map<String, dynamic>),
    );

Map<String, dynamic> _$APIResponseProcessdToJson(
        APIResponseProcessd instance) =>
    <String, dynamic>{
      'allProjects': instance.allProjects,
      'allScripts': instance.allScripts,
    };

APIResponseFailed _$APIResponseFailedFromJson(Map<String, dynamic> json) =>
    APIResponseFailed(
      missingToken: json['missingToken'] == null
          ? null
          : APIError.fromJson(json['missingToken'] as Map<String, dynamic>),
      emptyToken: json['emptyToken'] == null
          ? null
          : APIError.fromJson(json['emptyToken'] as Map<String, dynamic>),
      notLoggedIn: json['notLoggedIn'] == null
          ? null
          : APIError.fromJson(json['notLoggedIn'] as Map<String, dynamic>),
      internalServerError: json['internalServerError'] == null
          ? null
          : APIError.fromJson(
              json['internalServerError'] as Map<String, dynamic>),
    );

Map<String, dynamic> _$APIResponseFailedToJson(APIResponseFailed instance) =>
    <String, dynamic>{
      'missingToken': instance.missingToken,
      'emptyToken': instance.emptyToken,
      'notLoggedIn': instance.notLoggedIn,
      'internalServerError': instance.internalServerError,
    };

AllProjectsResponse _$AllProjectsResponseFromJson(Map<String, dynamic> json) =>
    AllProjectsResponse(
      processed: json['processed'] == null
          ? null
          : AllProjectsResponseProcessed.fromJson(
              json['processed'] as Map<String, dynamic>),
      failed: json['failed'] == null
          ? null
          : AllProjectsResponseFailed.fromJson(
              json['failed'] as Map<String, dynamic>),
    );

Map<String, dynamic> _$AllProjectsResponseToJson(
        AllProjectsResponse instance) =>
    <String, dynamic>{
      'processed': instance.processed,
      'failed': instance.failed,
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

AllProjectsResponseFailed _$AllProjectsResponseFailedFromJson(
        Map<String, dynamic> json) =>
    AllProjectsResponseFailed(
      cantReadProjects: json['cantReadProjects'] == null
          ? null
          : APIError.fromJson(json['cantReadProjects'] as Map<String, dynamic>),
      aProjectIsMissing: json['aProjectIsMissing'] == null
          ? null
          : APIError.fromJson(
              json['aProjectIsMissing'] as Map<String, dynamic>),
    );

Map<String, dynamic> _$AllProjectsResponseFailedToJson(
        AllProjectsResponseFailed instance) =>
    <String, dynamic>{
      'cantReadProjects': instance.cantReadProjects,
      'aProjectIsMissing': instance.aProjectIsMissing,
    };

AllScriptsResponse _$AllScriptsResponseFromJson(Map<String, dynamic> json) =>
    AllScriptsResponse(
      processed: json['processed'] == null
          ? null
          : AllScriptsResponseProcessed.fromJson(
              json['processed'] as Map<String, dynamic>),
      failed: json['failed'] == null
          ? null
          : AllScriptsResponseFailed.fromJson(
              json['failed'] as Map<String, dynamic>),
    );

Map<String, dynamic> _$AllScriptsResponseToJson(AllScriptsResponse instance) =>
    <String, dynamic>{
      'processed': instance.processed,
      'failed': instance.failed,
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

AllScriptsResponseFailed _$AllScriptsResponseFailedFromJson(
        Map<String, dynamic> json) =>
    AllScriptsResponseFailed(
      cantReadScripts: json['cantReadScripts'] == null
          ? null
          : APIError.fromJson(json['cantReadScripts'] as Map<String, dynamic>),
      aScriptIsMissing: json['aScriptIsMissing'] == null
          ? null
          : APIError.fromJson(json['aScriptIsMissing'] as Map<String, dynamic>),
    );

Map<String, dynamic> _$AllScriptsResponseFailedToJson(
        AllScriptsResponseFailed instance) =>
    <String, dynamic>{
      'cantReadScripts': instance.cantReadScripts,
      'aScriptIsMissing': instance.aScriptIsMissing,
    };
