import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/UserBlocs.dart';

class Me extends StatelessWidget {
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: BlocProvider.of<UserBloc>(context),
        builder: (context, snapshot) {
          if (!snapshot.hasData) {
            return Container(
                width: 20,
                child: Center(
                  child: CircularProgressIndicator(),
                ));
          }
          return ListView.builder(
            itemCount: snapshot.data.length,
            itemBuilder: (context, int index) {
              return Container(
                  child: Card(
                      elevation: 10,
                      margin: EdgeInsets.only(
                          left: 10.0, right: 10.0, top: 4, bottom: 4),
                      child: Padding(
                        padding: const EdgeInsets.all(8.0),
                        child: Row(
                          mainAxisAlignment: MainAxisAlignment.start,
                          children: <Widget>[
                            Image.asset('assets/test2.png',
                                width: 40, fit: BoxFit.contain),
                            SizedBox(
                              width: 10,
                            ),
                            Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: <Widget>[
                                Row(
                                  children: <Widget>[
                                    Text('${snapshot.data[index].uid}'),
                                    SizedBox(
                                      width: 10,
                                    ),
                                    Text('${snapshot.data[index].createdAt}'),
                                  ],
                                ),
                                SizedBox(
                                  width: 10,
                                ),
                                SizedBox(height: 5),
                                Text('${snapshot.data[index].postData}'),
                                SizedBox(height: 5),
                                InkWell(
                                    child: Text('Press for detail'),
                                    onTap: () {})
                              ],
                            )
                          ],
                        ),
                      )));
            },
          );
        });
  }
}


//class Me extends StatelessWidget {
//  Widget build(BuildContext context) {
//    return StreamBuilder(
//        stream: null,
//        builder: (context, snapshot) {
//          if (!snapshot.hasData) {
//            return Container(
//                width: 20,
//                child: Center(
//                  child: CircularProgressIndicator(),
//                ));
//          }
//          return ListView.builder(
//            itemCount: snapshot.data.length,
//            itemBuilder: (context, int index) {
//              return Container(
//                  child: Card(
//                      elevation: 10,
//                      margin: EdgeInsets.only(
//                          left: 10.0, right: 10.0, top: 4, bottom: 4),
//                      child: Padding(
//                        padding: const EdgeInsets.all(8.0),
//                        child: Row(
//                          mainAxisAlignment: MainAxisAlignment.start,
//                          children: <Widget>[
//                            Image.asset('assets/test2.png',
//                                width: 40, fit: BoxFit.contain),
//                            SizedBox(
//                              width: 10,
//                            ),
//                            Column(
//                              crossAxisAlignment: CrossAxisAlignment.start,
//                              children: <Widget>[
//                                Row(
//                                  children: <Widget>[
//                                    Text('${snapshot.data[index].uid}'),
//                                    SizedBox(
//                                      width: 10,
//                                    ),
//                                    Text('${snapshot.data[index].createdAt}'),
//                                  ],
//                                ),
//                                SizedBox(
//                                  width: 10,
//                                ),
//                                SizedBox(height: 5),
//                                Text('${snapshot.data[index].postData}'),
//                                SizedBox(height: 5),
//                                InkWell(
//                                    child: Text('Press for detail'),
//                                    onTap: () {})
//                              ],
//                            )
//                          ],
//                        ),
//                      )));
//            },
//          );
//        });
//  }
//}
