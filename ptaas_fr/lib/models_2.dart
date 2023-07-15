import 'package:json_annotation/json_annotation.dart';

part 'models_2.g.dart';

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

@JsonSerializable()
class APIError {
  final String message;
  final String? reason;

  APIError({required this.message, required this.reason});

  factory APIError.fromJson(Map<String, dynamic> json) =>
      _$APIErrorFromJson(json);

  Map<String, dynamic> toJson() => _$APIErrorToJson(this);
}

// Responses

@JsonSerializable()
class APIResponse {
  APIResponseProcessd? processed;
  APIResponseFailed? failed;

  APIResponse({this.processed, this.failed});

  factory APIResponse.fromJson(Map<String, dynamic> json) =>
      _$APIResponseFromJson(json);

  Map<String, dynamic> toJson() => _$APIResponseToJson(this);
}

@JsonSerializable()
class APIResponseProcessd {
  AllProjectsResponse? allProjects;
  AllScriptsResponse? allScripts;

  APIResponseProcessd({this.allProjects, this.allScripts});

  factory APIResponseProcessd.fromJson(Map<String, dynamic> json) =>
      _$APIResponseProcessdFromJson(json);

  Map<String, dynamic> toJson() => _$APIResponseProcessdToJson(this);
}

@JsonSerializable()
class APIResponseFailed {
  APIError? missingToken;
  APIError? emptyToken;
  APIError? notLoggedIn;
  APIError? internalServerError;

  APIResponseFailed(
      {this.missingToken,
      this.emptyToken,
      this.notLoggedIn,
      this.internalServerError});

  factory APIResponseFailed.fromJson(Map<String, dynamic> json) =>
      _$APIResponseFailedFromJson(json);

  Map<String, dynamic> toJson() => _$APIResponseFailedToJson(this);
}

// Projects

@JsonSerializable()
class AllProjectsResponse {
  AllProjectsResponseProcessed? processed;
  AllProjectsResponseFailed? failed;

  AllProjectsResponse({this.processed, this.failed});

  factory AllProjectsResponse.fromJson(Map<String, dynamic> json) =>
      _$AllProjectsResponseFromJson(json);

  Map<String, dynamic> toJson() => _$AllProjectsResponseToJson(this);
}

@JsonSerializable()
class AllProjectsResponseProcessed {
  List<Project> projects;

  AllProjectsResponseProcessed({required this.projects});

  factory AllProjectsResponseProcessed.fromJson(Map<String, dynamic> json) =>
      _$AllProjectsResponseProcessedFromJson(json);

  Map<String, dynamic> toJson() => _$AllProjectsResponseProcessedToJson(this);
}

@JsonSerializable()
class AllProjectsResponseFailed {
  APIError? cantReadProjects;
  APIError? aProjectIsMissing;

  AllProjectsResponseFailed({this.cantReadProjects, this.aProjectIsMissing});

  factory AllProjectsResponseFailed.fromJson(Map<String, dynamic> json) =>
      _$AllProjectsResponseFailedFromJson(json);

  Map<String, dynamic> toJson() => _$AllProjectsResponseFailedToJson(this);
}

// Scripts

@JsonSerializable()
class AllScriptsResponse {
  AllScriptsResponseProcessed? processed;
  AllScriptsResponseFailed? failed;

  AllScriptsResponse({this.processed, this.failed});

  factory AllScriptsResponse.fromJson(Map<String, dynamic> json) =>
      _$AllScriptsResponseFromJson(json);

  Map<String, dynamic> toJson() => _$AllScriptsResponseToJson(this);
}

@JsonSerializable()
class AllScriptsResponseProcessed {
  List<Script> scripts;

  AllScriptsResponseProcessed({required this.scripts});

  factory AllScriptsResponseProcessed.fromJson(Map<String, dynamic> json) =>
      _$AllScriptsResponseProcessedFromJson(json);

  Map<String, dynamic> toJson() => _$AllScriptsResponseProcessedToJson(this);
}

@JsonSerializable()
class AllScriptsResponseFailed {
  APIError? cantReadScripts;
  APIError? aScriptIsMissing;

  AllScriptsResponseFailed({this.cantReadScripts, this.aScriptIsMissing});

  factory AllScriptsResponseFailed.fromJson(Map<String, dynamic> json) =>
      _$AllScriptsResponseFailedFromJson(json);

  Map<String, dynamic> toJson() => _$AllScriptsResponseFailedToJson(this);
}
