import 'package:flutter/foundation.dart';
import 'package:json_annotation/json_annotation.dart';
part 'models.g.dart';

enum APIResponseType {
  @JsonValue('GeneralResponse')
  generalResponse,
  @JsonValue('AllProjectsResponse')
  allProjectsResponse,
  @JsonValue('AllScriptsResponse')
  allScriptsResponse,
}

enum APIGeneralResponseErrorType {
  @JsonValue('APIKeyIsMissing')
  apiKeyIsMissing,
  @JsonValue('APIKeyIsInvalid')
  apiKeyIsInvalid,
}

@JsonSerializable(fieldRename: FieldRename.snake)
class APIResponse<D, E> {
  APIResponseType responseType;
  @JsonKey(fromJson: _dataFromJson, toJson: _dataToJson)
  D? data;
  @JsonKey(fromJson: APIResponseError.fromJson, toJson: APIResponseError.toJson)
  APIResponseError<E>? error;

  APIResponse({
    required this.responseType,
    this.data,
    this.error,
  });

  static D? _dataFromJson<D>(dynamic json) {
    if (json == null) {
      return null;
    }
    return json as D;
  }

  static dynamic _dataToJson<D>(D? data) => data;
}

@JsonSerializable(fieldRename: FieldRename.snake)
class APIResponseError<E> {
  @JsonKey(fromJson: _errorTypeFromJson, toJson: _errorTypeToJson)
  E errorType;
  String errorMessage;

  APIResponseError({
    required this.errorType,
    required this.errorMessage,
  });

  static APIResponseError<E>? fromJson<E>(Map<String, dynamic> json) {
    try {
      return APIResponseError<E>(
        errorType: json['error_type'] as E,
        errorMessage: json['error_message'] as String,
      );
    } catch (e) {
      return null;
    }
  }

  static Map<String, dynamic> toJson<E>(APIResponseError<E>? instance) =>
      <String, dynamic>{
        'error_type': instance!.errorType,
        'error_message': instance.errorMessage,
      };

  static E _errorTypeFromJson<E>(dynamic json) {
    if (json == null) {
      return null as E;
    }
    return json as E;
  }

  static dynamic _errorTypeToJson<E>(E errorType) => errorType;
}

@JsonSerializable(fieldRename: FieldRename.snake)
class Project {
  String id;
  bool installed;
  List<Script> scripts;

  Project({
    required this.id,
    required this.installed,
    required this.scripts,
  });
}

@JsonSerializable(fieldRename: FieldRename.snake)
class Script {
  String id;

  Script({
    required this.id,
  });
}

@JsonSerializable(fieldRename: FieldRename.snake)
class AllProjectsResponseData {
  List<Project> projects;

  AllProjectsResponseData({
    required this.projects,
  });
}

enum AllProjectsResponseErrorType {
  @JsonValue('CantReadProjects')
  cantReadProjects,
  @JsonValue('AProjectIsMissing')
  aProjectIsMissing,
}

@JsonSerializable(fieldRename: FieldRename.snake)
class AllScriptsResponseData {
  List<Script> scripts;

  AllScriptsResponseData({
    required this.scripts,
  });
}

enum AllScriptsResponseErrorType {
  @JsonValue('CantReadScripts')
  cantReadScripts,
  @JsonValue('AScriptIsMissing')
  aScriptIsMissing,
  @JsonValue('CorrespondingProjectIsMissing')
  correspondingProjectIsMissing,
}
