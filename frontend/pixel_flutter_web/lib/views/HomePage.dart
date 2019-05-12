import 'package:flutter_web/gestures.dart';
import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_web/widgets.dart';

import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter_web/blocs/UserBloc/UserBloc.dart';
import 'package:pixel_flutter_web/blocs/UserBlocs.dart';
import 'package:pixel_flutter_web/components/FloatingAppBar.dart';
import 'package:pixel_flutter_web/components/UserDrawer.dart';

import 'package:pixel_flutter_web/style/text.dart';
import 'package:pixel_flutter_web/style/colors.dart';

const BREAK_POINT_WIDTH = 900.0;

class HomePage extends StatefulWidget {
  HomePage({Key key, this.title}) : super(key: key);

  final String title;

  @override
  _HomePageState createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  @override
  Widget build(BuildContext context) {
    return WillPopScope(
      onWillPop: onWillPop,
      child: BlocBuilder(
          bloc: BlocProvider.of<UserBloc>(context),
          builder: (context, userState) {
            return Scaffold(
              endDrawer: userState is UserLoaded ? UserDrawer() : null,
              body: BlocListener(
                bloc: BlocProvider.of<ErrorBloc>(context),
                listener: (BuildContext context, ErrorState state) async {
                  snackbarController(context, state);
                },
                child: CustomScrollView(
                  slivers: <Widget>[
                    FloatingAppBar(
                      title: "PixelWeb example",
                    ),
                    SliverFillViewport(
                      delegate: SliverChildBuilderDelegate((context, index) {
                        return Row(
                            mainAxisAlignment: MainAxisAlignment.center,
                            children: <Widget>[
                              Container(
                                width: MediaQuery.of(context).size.width > 700
                                    ? 700
                                    : MediaQuery.of(context).size.width,
                                color: Colors.amber,
                              ),
                              MediaQuery.of(context).size.width >
                                      BREAK_POINT_WIDTH
                                  ? Container(
                                      width: 200,
                                      color: Colors.black12,
                                    )
                                  : Container()
                            ]);
                      }, childCount: 1),
                    )
                  ],
                ),
              ),
            );
          }),
    );
  }

  Future<bool> onWillPop() {
    return showDialog(
        context: context,
        builder: (context) => AlertDialog(
              title: Text('Leaving?'),
              content: Text('Do you want to exit the app'),
              actions: <Widget>[
                FlatButton(
                  onPressed: () => Navigator.pop(context, false),
                  child: Text(
                    'Cancel',
                    style: recoverButtonStyle,
                  ),
                ),
                RaisedButton(
                  color: primaryColor,
                  onPressed: () => Navigator.pop(context, true),
                  child: Text(
                    'Confirm',
                    style: submitButtonStyle.copyWith(fontSize: 16),
                  ),
                )
              ],
            ));
  }

  snackbarController(BuildContext context, ErrorState state) async {
    if (state is NoSnack) {
      Scaffold.of(context).hideCurrentSnackBar();
    } else if (state is ShowSuccess) {
      Scaffold.of(context).showSnackBar(SnackBar(
        duration: Duration(seconds: 2),
        backgroundColor: Colors.green,
        content: Text(
          state.success,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 25, fontWeight: FontWeight.bold),
        ),
      ));
    } else if (state is ShowError) {
      Scaffold.of(context).showSnackBar(SnackBar(
        duration: Duration(seconds: 2),
        backgroundColor: Colors.deepOrangeAccent,
        content: Text(
          state.error,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 25, fontWeight: FontWeight.bold),
        ),
      ));
    }
  }
}
