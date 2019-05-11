import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_web/widgets.dart';

import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter_web/components/FloatingAppBar.dart';

const BREAK_POINT_WIDTH = 600.0;

class HomePage extends StatelessWidget {
  HomePage({Key key, this.title}) : super(key: key);

  final String title;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: BlocListener(
        bloc: BlocProvider.of<ErrorBloc>(context),
        listener: (BuildContext context, ErrorState state) async {
          snackbarController(context, state);
        },
        child: CustomScrollView(
          slivers: <Widget>[
            FloatingAppBar(),
            SliverFillViewport(
              delegate: SliverChildBuilderDelegate((context, index) {
                return Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: <Widget>[
                      Container(
                        width: MediaQuery.of(context).size.width > BREAK_POINT_WIDTH
                            ? MediaQuery.of(context).size.width / 2
                            : MediaQuery.of(context).size.width,
                        color: Colors.amber,
                      ),
                      MediaQuery.of(context).size.width > BREAK_POINT_WIDTH
                          ? Container(
                              width: 300,
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
