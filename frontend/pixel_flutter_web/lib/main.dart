import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter_web/blocs/UserBlocs.dart';

import 'package:pixel_flutter_web/views/HomePage.dart';

void main() => runApp(MyApp());

class MyApp extends StatelessWidget {
  final ErrorBloc errorBloc = ErrorBloc();
  final UserBloc userBloc = UserBloc();

  @override
  Widget build(BuildContext context) {
    userBloc.dispatch(UserInit());
    return BlocProviderTree(
      blocProviders: [
        BlocProvider<ErrorBloc>(bloc: errorBloc),
        BlocProvider<UserBloc>(bloc: userBloc),
      ],
      child: MaterialApp(
        title: 'Pixel Flutter Web',
        debugShowCheckedModeBanner: false,
        theme: ThemeData(
            brightness: Brightness.light,
            primarySwatch: Colors.blue,
            accentColor: Colors.deepPurple),
        home: HomePage(title: 'PixelShare'),
      ),
    );
  }
}
