import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter/blocs/UserBlocs.dart';

import 'package:pixel_flutter/Views/TopicsPage.dart';
import 'package:pixel_flutter/Views/AutenticationPage.dart';

class HomePage extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final _userBloc = BlocProvider.of<UserBloc>(context);
    return BlocBuilder(
        bloc: _userBloc,
        builder: (BuildContext context, UserState state) {
          if (state is AppStarted) {
            _userBloc.dispatch(UserInit());
            return Center(child: Container(child: CircularProgressIndicator()));
          }
          if (state is UserLoaded) {
            return TopicsPage();
          }
          if (state is Loading) {
            return Center(child: Container(child: CircularProgressIndicator()));
          }
          if (state is UserNone) {
            return AuthenticationPage();
          }
        });
  }
}