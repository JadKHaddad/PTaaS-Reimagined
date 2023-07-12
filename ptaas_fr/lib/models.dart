import 'package:json_annotation/json_annotation.dart';
part 'models.g.dart';

// dart run build_runner build

@JsonSerializable()
class Project {
  final String id;
  final bool installed;
  final List<Script> scripts;

  Project({required this.id, required this.installed, required this.scripts});

  factory Project.fromJson(Map<String, dynamic> json) =>
      _$ProjectFromJson(json);

  Map<String, dynamic> toJson() => _$ProjectToJson(this);
}

@JsonSerializable()
class Script {
  final String id;

  Script({required this.id});

  factory Script.fromJson(Map<String, dynamic> json) => _$ScriptFromJson(json);

  Map<String, dynamic> toJson() => _$ScriptToJson(this);
}

enum AllScriptsResponseErrorType {
  cantReadScripts,
  aScriptIsMissing,
  correspondingProjectIsMissing,
}

@JsonSerializable()
class AllScriptsResponse {
  final List<Script> scripts;

  AllScriptsResponse({required this.scripts});

  factory AllScriptsResponse.fromJson(Map<String, dynamic> json) =>
      _$AllScriptsResponseFromJson(json);

  Map<String, dynamic> toJson() => _$AllScriptsResponseToJson(this);
}

enum AllProjectsResponseErrorType { cantReadProjects, aProjectIsMissing }

@JsonSerializable()
class AllProjectsResponseData {
  final List<Project> projects;

  AllProjectsResponseData({required this.projects});

  factory AllProjectsResponseData.fromJson(Map<String, dynamic> json) =>
      _$AllProjectsResponseDataFromJson(json);

  Map<String, dynamic> toJson() => _$AllProjectsResponseDataToJson(this);
}

enum APIGerneralResponseErrorType {
  apiKeyIsMissing,
  apiKeyIsInvalid,
}

enum APIResponseType {
  gerneralResponse,
  allProjectsResponse,
  allScriptsResponse,
}

@JsonSerializable(genericArgumentFactories: true)
class APIResponseError<E> {
  final E errorType;
  final String errorMessage;

  APIResponseError({required this.errorType, required this.errorMessage});

  factory APIResponseError.fromJson(
          Map<String, dynamic> json, E Function(Object? json) fromJsonE) =>
      _$APIResponseErrorFromJson<E>(json, fromJsonE);

  Map<String, dynamic> toJson(Object? Function(E value) toJsonE) =>
      _$APIResponseErrorToJson<E>(this, toJsonE);
}

@JsonSerializable(genericArgumentFactories: true)
class APIResponse<D, E> {
  final bool success;
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
    E Function(Object? json) fromJsonE,
  ) =>
      _$APIResponseFromJson<D, E>(json, fromJsonD, fromJsonE);

  Map<String, dynamic> toJson(
    Object? Function(D value) toJsonD,
    Object? Function(E value) toJsonE,
  ) =>
      _$APIResponseToJson<D, E>(this, toJsonD, toJsonE);
}
