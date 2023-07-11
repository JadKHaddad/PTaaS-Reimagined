import 'package:flutter/foundation.dart';
import 'package:json_annotation/json_annotation.dart';
part 'models.g.dart';

@JsonSerializable(explicitToJson: true)
class Project {
  final String id;
  final bool installed;
  final List<Script> scripts;

  Project({required this.id, required this.installed, required this.scripts});

  factory Project.fromJson(Map<String, dynamic> json) =>
      _$ProjectFromJson(json);

  Map<String, dynamic> toJson() => _$ProjectToJson(this);
}

@JsonSerializable(explicitToJson: true)
class Script {
  final String id;

  Script({required this.id});

  factory Script.fromJson(Map<String, dynamic> json) => _$ScriptFromJson(json);

  Map<String, dynamic> toJson() => _$ScriptToJson(this);
}

enum AllScriptsResponseErrorType {
  @JsonValue('CantReadScripts')
  cantReadScripts,
  @JsonValue('AScriptIsMissing')
  aScriptIsMissing,
  @JsonValue('CorrespondingProjectIsMissing')
  correspondingProjectIsMissing,
}

@JsonSerializable(explicitToJson: true)
class AllScriptsResponse {
  final List<Script> scripts;

  AllScriptsResponse({required this.scripts});

  factory AllScriptsResponse.fromJson(Map<String, dynamic> json) =>
      _$AllScriptsResponseFromJson(json);

  Map<String, dynamic> toJson() => _$AllScriptsResponseToJson(this);
}

enum AllProjectsResponseErrorType {
  @JsonValue('CantReadProjects')
  cantReadProjects,
  @JsonValue('AProjectIsMissing')
  aProjectIsMissing
}

@JsonSerializable(explicitToJson: true)
class AllProjectsResponseData {
  final List<Project> projects;

  AllProjectsResponseData({required this.projects});

  factory AllProjectsResponseData.fromJson(Map<String, dynamic> json) =>
      _$AllProjectsResponseDataFromJson(json);

  Map<String, dynamic> toJson() => _$AllProjectsResponseDataToJson(this);
}

enum APIGerneralResponseErrorType {
  @JsonValue('APIKeyIsMissing')
  apiKeyIsMissing,
  @JsonValue('APIKeyIsInvalid')
  apiKeyIsInvalid,
}

enum APIResponseType {
  @JsonValue('GerneralResponse')
  gerneralResponse,
  @JsonValue('AllProjectsResponse')
  allProjectsResponse,
  @JsonValue('AllScriptsResponse')
  allScriptsResponse,
}

// TODO: can use enums for error types instead of generics
@JsonSerializable(explicitToJson: true, genericArgumentFactories: true)
class APIResponseError<E> {
  @JsonKey(name: 'error_type')
  final E errorType;

  @JsonKey(name: 'error_message')
  final String errorMessage;

  APIResponseError({required this.errorType, required this.errorMessage});

  factory APIResponseError.fromJson(
          Map<String, dynamic> json, E Function(Object? json) fromJsonE) =>
      _$APIResponseErrorFromJson<E>(json, fromJsonE);

  Map<String, dynamic> toJson(Object Function(E) toJsonE) =>
      _$APIResponseErrorToJson<E>(this, toJsonE);
}

@JsonSerializable(explicitToJson: true, genericArgumentFactories: true)
class APIResponse<D, E> {
  final bool success;
  @JsonKey(name: 'response_type')
  final APIResponseType responseType;
  final D? data;
  final APIResponseError<E>? error;

  APIResponse(
      {required this.success,
      required this.responseType,
      required this.data,
      required this.error});

  factory APIResponse.fromJson(
          Map<String, dynamic> json,
          D Function(Object? json) fromJsonD,
          E Function(Object? json) fromJsonE) =>
      _$APIResponseFromJson<D, E>(json, fromJsonD, fromJsonE);

  Map<String, dynamic> toJson(
          Object Function(D) toJsonD, Object Function(E) toJsonE) =>
      _$APIResponseToJson<D, E>(this, toJsonD, toJsonE);
}
