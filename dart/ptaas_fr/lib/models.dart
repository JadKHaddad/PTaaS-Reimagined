import 'dart:convert';

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

@JsonEnum(alwaysCreate: true)
enum AllScriptsResponseErrorType {
  cantReadScripts,
  aScriptIsMissing,
  correspondingProjectIsMissing;

  static AllScriptsResponseErrorType fromString(String value) {
    return $enumDecode(_$AllScriptsResponseErrorTypeEnumMap, value);
  }
}

@JsonSerializable()
class AllScriptsResponse {
  final List<Script> scripts;

  AllScriptsResponse({required this.scripts});

  factory AllScriptsResponse.fromJson(Map<String, dynamic> json) =>
      _$AllScriptsResponseFromJson(json);

  Map<String, dynamic> toJson() => _$AllScriptsResponseToJson(this);
}

@JsonEnum(alwaysCreate: true)
enum AllProjectsResponseErrorType {
  cantReadProjects,
  aProjectIsMissing;

  static AllProjectsResponseErrorType fromString(String value) {
    return $enumDecode(_$AllProjectsResponseErrorTypeEnumMap, value);
  }
}

@JsonSerializable()
class AllProjectsResponseData {
  final List<Project> projects;

  AllProjectsResponseData({required this.projects});

  factory AllProjectsResponseData.fromJson(Map<String, dynamic> json) =>
      _$AllProjectsResponseDataFromJson(json);

  Map<String, dynamic> toJson() => _$AllProjectsResponseDataToJson(this);
}

@JsonEnum(alwaysCreate: true)
enum APIGerneralResponseErrorType {
  aPIKeyIsMissing,
  aPIKeyIsInvalid;

  factory APIGerneralResponseErrorType.fromString(String value) {
    return $enumDecode(_$APIGerneralResponseErrorTypeEnumMap, value);
  }
}

@JsonEnum(alwaysCreate: true)
enum APIResponseType {
  gerneralResponse,
  allProjectsResponse,
  allScriptsResponse;

  factory APIResponseType.fromString(String value) {
    return $enumDecode(_$APIResponseTypeEnumMap, value);
  }
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

void _doSomeAPIStuff() {
  String allProjectsResponseSuccessString =
      '{"success":true,"responseType":"allProjectsResponse","data":{"projects":[{"id":"project1","installed":true,"scripts":[{"id":"script1"},{"id":"script2"}]},{"id":"project2","installed":false,"scripts":[{"id":"script3"}]}]},"error":null}';
  String allProjectsResponseErrorString =
      '{"success":false,"responseType":"allProjectsResponse","data":null,"error":{"errorType":"cantReadProjects","errorMessage":"Failed to read projects."}}';
  String apiErrorResponseString =
      '{"success":false,"responseType":"gerneralResponse","data":null,"error":{"errorType":"aPIKeyIsMissing","errorMessage":"API key is missing."}}';

  Map<String, dynamic> json = jsonDecode(apiErrorResponseString);

  try {
    // if this one fails, the error is in an api error so we parse it as such
    APIResponse<AllProjectsResponseData?, AllProjectsResponseErrorType?>
        allProjectsResponse = APIResponse.fromJson(
            json,
            (json) =>
                AllProjectsResponseData.fromJson(json as Map<String, dynamic>),
            (json) => AllProjectsResponseErrorType.fromString(json as String));
    print("All projects response: $json");
  } catch (e) {
    APIResponse<Object?, APIGerneralResponseErrorType?> apiErrorResponse =
        APIResponse.fromJson(
            json,
            (json) =>
                null /* we don't care about the data because its never there!*/,
            (json) => APIGerneralResponseErrorType.fromString(json as String));
    print("API error response: $json");
  }
}
