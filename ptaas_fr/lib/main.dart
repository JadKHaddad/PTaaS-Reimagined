import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';

import 'models.dart';

void main() {
  runApp(const MainApp());
}

class MainApp extends StatelessWidget {
  const MainApp({super.key});

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
              (json) => AllProjectsResponseData.fromJson(
                  json as Map<String, dynamic>),
              (json) =>
                  AllProjectsResponseErrorType.fromString(json as String));
      print("All projects response: $json");
    } catch (e) {
      APIResponse<Object?, APIGerneralResponseErrorType?> apiErrorResponse =
          APIResponse.fromJson(
              json,
              (json) =>
                  null /* we don't care about the data because its never there!*/,
              (json) =>
                  APIGerneralResponseErrorType.fromString(json as String));
      print("API error response: $json");
    }
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        body: Center(
            child: TextButton(
                onPressed: _doSomeAPIStuff,
                child: const Text("Do some API stuff"))),
      ),
    );
  }
}
